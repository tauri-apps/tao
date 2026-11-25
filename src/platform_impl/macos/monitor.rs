// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{collections::VecDeque, fmt, ptr::NonNull};

use super::ffi::{self, CGRectContainsPoint};
use crate::{
  dpi::{PhysicalPosition, PhysicalSize},
  monitor::{MonitorHandle as RootMonitorHandle, VideoMode as RootVideoMode},
};
use core_foundation::{
  array::{CFArrayGetCount, CFArrayGetValueAtIndex},
  base::{CFRelease, TCFType},
  string::CFString,
};
use core_graphics::{
  display::{CGDirectDisplayID, CGDisplay, CGDisplayBounds},
  geometry::CGPoint,
};
use objc2::rc::Retained;
use objc2_app_kit::NSScreen;
use objc2_core_foundation::{CFRetained, CFUUID};
use objc2_foundation::{ns_string, MainThreadMarker, NSNumber};

#[derive(Clone)]
pub struct VideoMode {
  pub(crate) size: (u32, u32),
  pub(crate) bit_depth: u16,
  pub(crate) refresh_rate: u16,
  pub(crate) monitor: MonitorHandle,
  pub(crate) native_mode: NativeDisplayMode,
  display_id: CGDirectDisplayID,
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

pub struct NativeDisplayMode(pub ffi::CGDisplayModeRef);

unsafe impl Send for NativeDisplayMode {}

impl Drop for NativeDisplayMode {
  fn drop(&mut self) {
    unsafe {
      ffi::CGDisplayModeRelease(self.0);
    }
  }
}

impl Clone for NativeDisplayMode {
  fn clone(&self) -> Self {
    unsafe {
      ffi::CGDisplayModeRetain(self.0);
    }
    NativeDisplayMode(self.0)
  }
}

impl VideoMode {
  // The native configuration belongs to the display ID from which it was enumerated.
  pub(super) fn display_id(&self) -> Option<CGDirectDisplayID> {
    self
      .monitor
      .display_id()
      .filter(|&id| id == self.display_id)
  }

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

// Keep construction infallible when CoreGraphics cannot provide a UUID.
#[derive(Clone)]
pub struct MonitorHandle(Result<CFRetained<CFUUID>, CGDirectDisplayID>);

// CFUUID is immutable.
impl std::panic::UnwindSafe for MonitorHandle {}
impl std::panic::RefUnwindSafe for MonitorHandle {}

fn display_uuid(display_id: CGDirectDisplayID) -> Option<CFRetained<CFUUID>> {
  let ptr = NonNull::new(unsafe { ffi::CGDisplayCreateUUIDFromDisplayID(display_id) })?;
  // SAFETY: The Create function returns an owned (+1) reference.
  Some(unsafe { CFRetained::from_raw(ptr) })
}

impl PartialEq for MonitorHandle {
  fn eq(&self, other: &Self) -> bool {
    self.identity() == other.identity()
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
    self.identity().cmp(&other.identity())
  }
}

impl std::hash::Hash for MonitorHandle {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.identity().hash(state);
  }
}

pub fn available_monitors() -> VecDeque<MonitorHandle> {
  if let Ok(displays) = CGDisplay::active_displays() {
    let mut monitors = VecDeque::with_capacity(displays.len());
    for display in displays {
      monitors.push_back(MonitorHandle::new(display));
    }
    monitors
  } else {
    VecDeque::with_capacity(0)
  }
}

pub fn primary_monitor() -> MonitorHandle {
  MonitorHandle::new(CGDisplay::main().id)
}

// `from_point` get a monitor handle which contains the given point.
pub fn from_point(x: f64, y: f64) -> Option<MonitorHandle> {
  unsafe {
    for monitor in available_monitors() {
      let Some(display_id) = monitor.display_id() else {
        continue;
      };
      let bound = CGDisplayBounds(display_id);
      if CGRectContainsPoint(bound, CGPoint::new(x, y)) > 0 {
        return Some(monitor);
      }
    }
  }
  None
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
  fn identity(&self) -> Result<[u8; 16], CGDirectDisplayID> {
    self
      .0
      .as_ref()
      .map(|uuid| uuid.uuid_bytes().into())
      .map_err(|id| *id)
  }

  pub fn new(id: CGDirectDisplayID) -> Self {
    Self(display_uuid(id).ok_or(id))
  }

  pub(super) fn display_id(&self) -> Option<CGDirectDisplayID> {
    let id = match &self.0 {
      Ok(uuid) => unsafe { ffi::CGDisplayGetDisplayIDFromUUID(uuid) },
      Err(id) => *id,
    };
    // An offline UUID resolves to kCGNullDirectDisplay.
    (id != 0).then_some(id)
  }

  pub fn name(&self) -> Option<String> {
    let screen_num = CGDisplay::new(self.display_id()?).model_number();
    Some(format!("Monitor #{}", screen_num))
  }

  #[inline]
  pub fn native_identifier(&self) -> u32 {
    self.display_id().unwrap_or(0)
  }

  pub fn size(&self) -> PhysicalSize<u32> {
    let Some(display_id) = self.display_id() else {
      return PhysicalSize::new(0, 0);
    };
    let display = CGDisplay::new(display_id);
    let height = display.pixels_high();
    let width = display.pixels_wide();
    PhysicalSize::from_logical::<_, f64>((width as f64, height as f64), self.scale_factor())
  }

  #[inline]
  pub fn position(&self) -> PhysicalPosition<i32> {
    let Some(display_id) = self.display_id() else {
      return PhysicalPosition::new(0, 0);
    };
    let bounds = unsafe { CGDisplayBounds(display_id) };
    PhysicalPosition::from_logical::<_, f64>(
      (bounds.origin.x as f64, bounds.origin.y as f64),
      self.scale_factor(),
    )
  }

  pub fn scale_factor(&self) -> f64 {
    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    let screen = match self.ns_screen(mtm) {
      Some(screen) => screen,
      None => return 1.0, // default to 1.0 when we can't find the screen
    };
    NSScreen::backingScaleFactor(&screen)
  }

  pub fn video_modes(&self) -> impl Iterator<Item = RootVideoMode> {
    self
      .display_id()
      .map(|id| self.video_modes_for_display(id))
      .into_iter()
      .flatten()
  }

  fn video_modes_for_display(
    &self,
    display_id: CGDirectDisplayID,
  ) -> impl Iterator<Item = RootVideoMode> {
    let cv_refresh_rate = unsafe {
      let mut display_link = std::ptr::null_mut();
      assert_eq!(
        ffi::CVDisplayLinkCreateWithCGDisplay(display_id, &mut display_link),
        ffi::kCVReturnSuccess
      );
      let time = ffi::CVDisplayLinkGetNominalOutputVideoRefreshPeriod(display_link);
      ffi::CVDisplayLinkRelease(display_link);

      // This value is indefinite if an invalid display link was specified
      assert!(time.flags & ffi::kCVTimeIsIndefinite == 0);

      time.time_scale as i64 / time.time_value
    };

    let monitor = self.clone();

    unsafe {
      let modes = {
        let array = ffi::CGDisplayCopyAllDisplayModes(display_id, std::ptr::null());
        assert!(!array.is_null(), "failed to get list of display modes");
        let array_count = CFArrayGetCount(array);
        let modes: Vec<_> = (0..array_count)
          .map(move |i| {
            let mode = CFArrayGetValueAtIndex(array, i) as *mut _;
            ffi::CGDisplayModeRetain(mode);
            mode
          })
          .collect();
        CFRelease(array as *const _);
        modes
      };

      modes.into_iter().map(move |mode| {
        let cg_refresh_rate = ffi::CGDisplayModeGetRefreshRate(mode).round() as i64;

        // CGDisplayModeGetRefreshRate returns 0.0 for any display that
        // isn't a CRT
        let refresh_rate = if cg_refresh_rate > 0 {
          cg_refresh_rate
        } else {
          cv_refresh_rate
        };

        let pixel_encoding =
          CFString::wrap_under_create_rule(ffi::CGDisplayModeCopyPixelEncoding(mode)).to_string();
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
            ffi::CGDisplayModeGetPixelWidth(mode) as u32,
            ffi::CGDisplayModeGetPixelHeight(mode) as u32,
          ),
          refresh_rate: refresh_rate as u16,
          bit_depth,
          monitor: monitor.clone(),
          native_mode: NativeDisplayMode(mode),
          display_id,
        };

        RootVideoMode { video_mode }
      })
    }
  }

  pub(crate) fn ns_screen(&self, mtm: MainThreadMarker) -> Option<Retained<NSScreen>> {
    let uuid = self.0.as_ref().ok()?;
    NSScreen::screens(mtm).into_iter().find(|screen| {
      let display_id = get_display_id(screen);
      if let Some(other) = display_uuid(display_id) {
        *uuid == other
      } else {
        warn!("comparing against screen with invalid display ID {display_id}");
        false
      }
    })
  }
}

pub(super) fn get_display_id(screen: &NSScreen) -> CGDirectDisplayID {
  let key = ns_string!("NSScreenNumber");
  objc2::rc::autoreleasepool(|_| {
    screen
      .deviceDescription()
      .objectForKey(key)
      .expect("failed getting screen display id from device description")
      .downcast::<NSNumber>()
      .expect("NSScreenNumber must be NSNumber")
      .as_u32()
  })
}
