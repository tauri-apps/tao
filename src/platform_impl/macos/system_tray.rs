// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use super::menu::Menu;
use crate::{error::OsError, event_loop::EventLoopWindowTarget, menu::Menu as RootMenu};
use cocoa::{
  appkit::{NSButton, NSImage, NSSquareStatusItemLength, NSStatusBar, NSStatusItem},
  base::{id, nil},
  foundation::{NSAutoreleasePool, NSData, NSSize},
};

pub struct SystemTrayBuilder {
  system_tray: SystemTray,
}

impl SystemTrayBuilder {
  /// Creates a new SystemTray for platforms where this is appropriate.
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[inline]
  pub fn new(icon: Vec<u8>, tray_menu: RootMenu) -> Self {
    unsafe {
      let ns_status_bar = NSStatusBar::systemStatusBar(nil)
        .statusItemWithLength_(NSSquareStatusItemLength)
        .autorelease();

      Self {
        system_tray: SystemTray {
          icon,
          tray_menu: tray_menu.menu_platform,
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

      // set tray menu
      status_bar.setMenu_(self.system_tray.tray_menu.menu);
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
  pub(crate) tray_menu: Menu,
  pub(crate) ns_status_bar: id,
}

impl SystemTray {
  pub fn update_icon(&mut self, icon: Vec<u8>) {
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
