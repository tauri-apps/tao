// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use super::{
  app_state::AppState,
  event::EventWrapper,
  menu::Menu,
  util::{bottom_left_to_top_left_for_cursor, bottom_left_to_top_left_for_tray},
};
use crate::{
  dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
  error::OsError,
  event::{Event, Rectangle, TrayEvent},
  event_loop::EventLoopWindowTarget,
  system_tray::SystemTray as RootSystemTray,
};
use cocoa::{
  appkit::{
    NSButton, NSEventMask, NSEventModifierFlags, NSEventType, NSImage, NSSquareStatusItemLength,
    NSStatusBar, NSStatusItem, NSWindow,
  },
  base::{id, nil},
  foundation::{NSAutoreleasePool, NSData, NSPoint, NSSize},
};
use objc::{
  declare::ClassDecl,
  runtime::{Class, Object, Sel},
};
use std::sync::Once;

pub struct SystemTrayBuilder {
  pub(crate) system_tray: SystemTray,
}

impl SystemTrayBuilder {
  /// Creates a new SystemTray for platforms where this is appropriate.
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[inline]
  pub fn new(icon: Vec<u8>, tray_menu: Option<Menu>) -> Self {
    unsafe {
      let ns_status_bar = NSStatusBar::systemStatusBar(nil)
        .statusItemWithLength_(NSSquareStatusItemLength)
        .autorelease();

      Self {
        system_tray: SystemTray {
          icon,
          tray_menu,
          ns_status_bar,
        },
      }
    }
  }

  /// Builds the system tray.
  ///
  /// Possible causes of error include denied permission, incompatible system, and lack of memory.
  #[inline]
  pub fn build<T: 'static>(
    self,
    _window_target: &EventLoopWindowTarget<T>,
  ) -> Result<RootSystemTray, OsError> {
    unsafe {
      // use our existing status bar
      let status_bar = self.system_tray.ns_status_bar;

      // set our icon
      self.system_tray.create_button_with_icon();

      // attach menu only if provided
      if let Some(menu) = self.system_tray.tray_menu.clone() {
        // set tray menu
        status_bar.setMenu_(menu.menu);
      }

      // attach click event to our button
      let button = status_bar.button();
      let tray_target: id = msg_send![make_tray_class(), alloc];
      let tray_target: id = msg_send![tray_target, init];
      let _: () = msg_send![button, setAction: sel!(click:)];
      let _: () = msg_send![button, setTarget: tray_target];
      let _: () = msg_send![
        button,
        sendActionOn: NSEventMask::NSLeftMouseDownMask
          | NSEventMask::NSRightMouseDownMask
          | NSEventMask::NSKeyDownMask
      ];
    }

    Ok(RootSystemTray(self.system_tray))
  }
}

/// System tray is a status icon that can show popup menu. It is usually displayed on top right or bottom right of the screen.
///
/// ## Platform-specific
///
/// - **Linux:**: require `menu` feature flag. Otherwise, it's a no-op.
#[derive(Debug, Clone)]
pub struct SystemTray {
  pub(crate) icon: Vec<u8>,
  pub(crate) tray_menu: Option<Menu>,
  pub(crate) ns_status_bar: id,
}

impl SystemTray {
  pub fn set_icon(&mut self, icon: Vec<u8>) {
    // update our icon
    self.icon = icon;
    self.create_button_with_icon();
  }

  fn create_button_with_icon(&self) {
    const ICON_WIDTH: f64 = 18.0;
    const ICON_HEIGHT: f64 = 18.0;

    unsafe {
      let status_item = self.ns_status_bar;
      let button = status_item.button();

      // build our icon
      let nsdata = NSData::dataWithBytes_length_(
        nil,
        self.icon.as_ptr() as *const std::os::raw::c_void,
        self.icon.len() as u64,
      );

      let nsimage = NSImage::initWithData_(NSImage::alloc(nil), nsdata);
      let new_size = NSSize::new(ICON_WIDTH, ICON_HEIGHT);

      button.setImage_(nsimage);
      let _: () = msg_send![nsimage, setSize: new_size];
    }
  }
}

fn make_tray_class() -> *const Class {
  static mut TRAY_CLASS: *const Class = 0 as *const Class;
  static INIT: Once = Once::new();

  INIT.call_once(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("TaoTrayHandler", superclass).unwrap();
    decl.add_method(
      sel!(click:),
      perform_tray_click as extern "C" fn(&mut Object, _, id),
    );

    TRAY_CLASS = decl.register();
  });

  unsafe { TRAY_CLASS }
}

/// This will fire for an NSButton callback.
extern "C" fn perform_tray_click(_this: &mut Object, _: Sel, _sender: id) {
  unsafe {
    let app: id = msg_send![class!(NSApplication), sharedApplication];
    let current_event: id = msg_send![app, currentEvent];

    // icon position & size
    let window: id = msg_send![current_event, window];
    let frame = NSWindow::frame(window);
    let scale_factor = NSWindow::backingScaleFactor(window) as f64;
    let position: PhysicalPosition<f64> = LogicalPosition::new(
      frame.origin.x as f64,
      bottom_left_to_top_left_for_tray(frame),
    )
    .to_physical(scale_factor);

    let logical: LogicalSize<f64> = (frame.size.width as f64, frame.size.height as f64).into();
    let size: PhysicalSize<f64> = logical.to_physical(scale_factor);

    // cursor position
    let mouse_location: NSPoint = msg_send![class!(NSEvent), mouseLocation];
    // what type of click?
    let event_mask: NSEventType = msg_send![current_event, type];
    // grab the modifier flag, to make sure the ctrl + left click = right click
    let key_code: NSEventModifierFlags = msg_send![current_event, modifierFlags];

    let click_type = match event_mask {
      // left click + control key
      NSEventType::NSLeftMouseDown if key_code.contains(NSEventModifierFlags::NSControlKeyMask) => {
        Some(TrayEvent::RightClick)
      }
      NSEventType::NSLeftMouseDown => Some(TrayEvent::LeftClick),
      NSEventType::NSRightMouseDown => Some(TrayEvent::RightClick),
      _ => None,
    };

    if let Some(event) = click_type {
      let event = Event::TrayEvent {
        bounds: Rectangle { position, size },
        position: PhysicalPosition::new(
          mouse_location.x,
          bottom_left_to_top_left_for_cursor(mouse_location),
        ),
        event,
      };

      AppState::queue_event(EventWrapper::StaticEvent(event));
    }
  }
}
