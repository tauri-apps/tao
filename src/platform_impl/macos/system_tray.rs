// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{error::OsError, event_loop::EventLoopWindowTarget, event::{Event, CursorClick}};
use cocoa::{
  appkit::{NSButton, NSImage, NSSquareStatusItemLength, NSStatusBar, NSStatusItem},
  base::{id, nil},
  foundation::{NSAutoreleasePool, NSData, NSSize},
};
use objc::{
  declare::ClassDecl,
  runtime::{Class, Object, Sel},
};
use super::{menu::Menu, app_state::AppState, event::EventWrapper};
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
  ) -> Result<SystemTray, OsError> {
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
      let _: () = msg_send![button, setAction: sel!(perform:)];
      let _: () = msg_send![button, setTarget: tray_target];
    }

    Ok(self.system_tray)
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
    decl.add_method(sel!(perform:), perform as extern fn (&mut Object, _, id));

    TRAY_CLASS = decl.register();
  });

  unsafe { TRAY_CLASS }
}

/// This will fire for an NSButton callback.
extern fn perform(_this: &mut Object, _: Sel, _sender: id) {
  let event = Event::TrayClick(CursorClick::Left);
  AppState::queue_event(EventWrapper::StaticEvent(event));
}
