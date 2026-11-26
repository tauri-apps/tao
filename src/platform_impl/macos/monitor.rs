// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  collections::VecDeque,
  fmt,
  ptr::{self, NonNull},
};

use super::ffi;
use crate::{
  dpi::{PhysicalPosition, PhysicalSize},
  monitor::{MonitorHandle as RootMonitorHandle, VideoMode as RootVideoMode},
};
use objc2::rc::Retained;
use objc2_app_kit::NSScreen;
use objc2_core_foundation::{CFArray, CFRetained, CGPoint};
use objc2_core_graphics::{
  CGDirectDisplayID, CGDisplayBounds, CGDisplayCopyAllDisplayModes, CGDisplayMode,
  CGDisplayModelNumber, CGDisplayPixelsHigh, CGDisplayPixelsWide, CGError, CGGetActiveDisplayList,
  CGMainDisplayID, CGRectContainsPoint,
};
#[allow(deprecated)]
use objc2_core_video::{kCVReturnSuccess, CVDisplayLink, CVTimeFlags};
use objc2_foundation::{ns_string, MainThreadMarker, NSNumber};

#[derive(Clone)]
pub struct VideoMode {
  pub(crate) size: (u32, u32),
  pub(crate) bit_depth: u16,
  pub(crate) refresh_rate: u16,
  pub(crate) monitor: MonitorHandle,
  pub(crate) native_mode: NativeDisplayMode,
}

impl PartialEq for VideoMode {
  fn eq(&self, other: &Self) -> bool {
    self.size == other.size
      && self.bit_depth == other.bit_depth
      && self.refresh_rate == other.refresh_rate
      && self.monitor == other.monitor
  }
}

impl Eq for VideoMode {}

impl std::hash::Hash for VideoMode {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.size.hash(state);
    self.bit_depth.hash(state);
    self.refresh_rate.hash(state);
    self.monitor.hash(state);
  }
}

impl std::fmt::Debug for VideoMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("VideoMode")
      .field("size", &self.size)
      .field("bit_depth", &self.bit_depth)
      .field("refresh_rate", &self.refresh_rate)
      .field("monitor", &self.monitor)
      .finish()
  }
}

#[derive(Clone)]
pub struct NativeDisplayMode(pub CFRetained<CGDisplayMode>);

unsafe impl Send for NativeDisplayMode {}

impl VideoMode {
  pub fn size(&self) -> PhysicalSize<u32> {
    self.size.into()
  }

  pub fn bit_depth(&self) -> u16 {
    self.bit_depth
  }

  pub fn refresh_rate(&self) -> u16 {
    self.refresh_rate
  }

  pub fn monitor(&self) -> RootMonitorHandle {
    RootMonitorHandle {
      inner: self.monitor.clone(),
    }
  }
}

#[derive(Clone)]
pub struct MonitorHandle(CGDirectDisplayID);

// `CGDirectDisplayID` changes on video mode change, so we cannot rely on that
// for comparisons, but we can use `CGDisplayCreateUUIDFromDisplayID` to get an
// unique identifier that persists even across system reboots
impl PartialEq for MonitorHandle {
  fn eq(&self, other: &Self) -> bool {
    self.uuid() == other.uuid()
  }
}

impl Eq for MonitorHandle {}

impl PartialOrd for MonitorHandle {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for MonitorHandle {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.uuid().cmp(&other.uuid())
  }
}

impl std::hash::Hash for MonitorHandle {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.uuid().hash(state);
  }
}

pub fn available_monitors() -> VecDeque<MonitorHandle> {
  let mut count = 0;
  // Passing a null display buffer queries the number of active displays.
  let status = unsafe { CGGetActiveDisplayList(0, ptr::null_mut(), &mut count) };
  if status != CGError::Success || count == 0 {
    return VecDeque::new();
  }

  let mut displays = vec![0; count as usize];
  // The display buffer can hold up to count IDs.
  let status = unsafe { CGGetActiveDisplayList(count, displays.as_mut_ptr(), &mut count) };
  if status != CGError::Success {
    return VecDeque::new();
  }

  displays.truncate(count as usize);
  displays.into_iter().map(MonitorHandle).collect()
}

pub fn primary_monitor() -> MonitorHandle {
  MonitorHandle(CGMainDisplayID())
}

// `from_point` get a monitor handle which contains the given point.
pub fn from_point(x: f64, y: f64) -> Option<MonitorHandle> {
  let point = CGPoint::new(x, y);
  available_monitors()
    .into_iter()
    .find(|monitor| CGRectContainsPoint(CGDisplayBounds(monitor.0), point))
}

impl fmt::Debug for MonitorHandle {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // TODO: Do this using the proper fmt API
    #[derive(Debug)]
    #[allow(dead_code)] // none of the member reads in Debug::fmt are detected
    struct MonitorHandle {
      name: Option<String>,
      native_identifier: u32,
      size: PhysicalSize<u32>,
      position: PhysicalPosition<i32>,
      scale_factor: f64,
    }

    let monitor_id_proxy = MonitorHandle {
      name: self.name(),
      native_identifier: self.native_identifier(),
      size: self.size(),
      position: self.position(),
      scale_factor: self.scale_factor(),
    };

    monitor_id_proxy.fmt(f)
  }
}

impl MonitorHandle {
  fn uuid(&self) -> Option<u128> {
    let uuid = unsafe { ffi::CGDisplayCreateUUIDFromDisplayID(self.0) };
    // The Create function transfers a retain.
    let uuid = unsafe { CFRetained::from_raw(NonNull::new(uuid)?) };
    Some(u128::from_ne_bytes(uuid.uuid_bytes().into()))
  }

  pub fn new(id: CGDirectDisplayID) -> Self {
    MonitorHandle(id)
  }

  pub fn name(&self) -> Option<String> {
    let screen_num = CGDisplayModelNumber(self.0);
    Some(format!("Monitor #{}", screen_num))
  }

  #[inline]
  pub fn native_identifier(&self) -> u32 {
    self.0
  }

  pub fn size(&self) -> PhysicalSize<u32> {
    let height = CGDisplayPixelsHigh(self.0);
    let width = CGDisplayPixelsWide(self.0);
    PhysicalSize::from_logical::<_, f64>((width as f64, height as f64), self.scale_factor())
  }

  #[inline]
  pub fn position(&self) -> PhysicalPosition<i32> {
    let bounds = CGDisplayBounds(self.native_identifier());
    PhysicalPosition::from_logical::<_, f64>(
      (bounds.origin.x as f64, bounds.origin.y as f64),
      self.scale_factor(),
    )
  }

  pub fn scale_factor(&self) -> f64 {
    let screen = match self.ns_screen() {
      Some(screen) => screen,
      None => return 1.0, // default to 1.0 when we can't find the screen
    };
    NSScreen::backingScaleFactor(&screen)
  }

  #[allow(deprecated)]
  pub fn video_modes(&self) -> impl Iterator<Item = RootVideoMode> {
    let cv_refresh_rate = unsafe {
      let mut display_link = std::ptr::null_mut();
      assert_eq!(
        CVDisplayLink::create_with_cg_display(self.0, NonNull::from(&mut display_link)),
        kCVReturnSuccess
      );
      let display_link = CFRetained::from_raw(NonNull::new(display_link).unwrap());
      let time = display_link.nominal_output_video_refresh_period();

      // This value is indefinite if an invalid display link was specified
      assert!(time.flags & CVTimeFlags::IsIndefinite.0 == 0);

      time.timeScale as i64 / time.timeValue
    };

    let monitor = self.clone();

    let modes = unsafe { CGDisplayCopyAllDisplayModes(self.0, None) }
      .expect("failed to get list of display modes");
    // Core Graphics documents the array elements as CGDisplayMode objects.
    let modes = unsafe { CFRetained::cast_unchecked::<CFArray<CGDisplayMode>>(modes) };

    modes.into_iter().map(move |mode| {
      let cg_refresh_rate = CGDisplayMode::refresh_rate(Some(&mode)).round() as i64;

      // CGDisplayModeGetRefreshRate returns 0.0 for any display that
      // isn't a CRT
      let refresh_rate = if cg_refresh_rate > 0 {
        cg_refresh_rate
      } else {
        cv_refresh_rate
      };

      let pixel_encoding = CGDisplayMode::pixel_encoding(Some(&mode))
        .unwrap()
        .to_string();
      let bit_depth = if pixel_encoding.eq_ignore_ascii_case(ffi::IO32BitDirectPixels) {
        32
      } else if pixel_encoding.eq_ignore_ascii_case(ffi::IO16BitDirectPixels) {
        16
      } else if pixel_encoding.eq_ignore_ascii_case(ffi::kIO30BitDirectPixels) {
        30
      } else {
        unimplemented!()
      };

      let video_mode = VideoMode {
        size: (
          CGDisplayMode::pixel_width(Some(&mode)) as u32,
          CGDisplayMode::pixel_height(Some(&mode)) as u32,
        ),
        refresh_rate: refresh_rate as u16,
        bit_depth,
        monitor: monitor.clone(),
        native_mode: NativeDisplayMode(mode),
      };

      RootVideoMode { video_mode }
    })
  }

  pub(crate) fn ns_screen(&self) -> Option<Retained<NSScreen>> {
    // SAFETY: TODO.
    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    let uuid = self.uuid()?;
    let screens = NSScreen::screens(mtm);
    let key = ns_string!("NSScreenNumber");
    for screen in screens {
      let device_description = NSScreen::deviceDescription(&screen);
      if let Some(value) = device_description.objectForKey(key) {
        let value = value.downcast::<NSNumber>().unwrap();
        let other_native_id = value.unsignedIntegerValue();
        let other = Self(other_native_id as CGDirectDisplayID);
        if Some(uuid) == other.uuid() {
          return Some(screen);
        }
      }
    }
    None
  }
}
