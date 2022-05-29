// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

//! **UNSTABLE** -- The `SystemTray` struct and associated types.
//!
//! Use [SystemTrayBuilder][tray_builder] to create your tray instance.
//!
//! [ContextMenu][context_menu] is used to created a Window menu on Windows and Linux. On macOS it's used in the menubar.
//!
//! ```rust,ignore
//! # let icon_rgba = Vec::<u8>::new();
//! # let icon_width = 0;
//! # let icon_height = 0;
//! let mut tray_menu = ContextMenu::new();
//! let icon = Icon::from_rgba(icon_rgba, icon_width, icon_height);
//!
//! tray_menu.add_item(MenuItemAttributes::new("My menu item"));
//!
//! let mut system_tray = SystemTrayBuilder::new(icon, Some(tray_menu))
//!   .build(&event_loop)
//!   .unwrap();
//! ```
//!
//! # Linux
//! A menu is required or the tray return an error containing `assertion 'G_IS_DBUS_CONNECTION (connection)'`.
//!
//! [tray_builder]: crate::system_tray::SystemTrayBuilder
//! [menu_bar]: crate::menu::MenuBar
//! [context_menu]: crate::menu::ContextMenu

use crate::{
  error::OsError,
  event_loop::EventLoopWindowTarget,
  menu::ContextMenu,
  platform_impl::{
    SystemTray as SystemTrayPlatform, SystemTrayBuilder as SystemTrayBuilderPlatform,
  },
};

pub use crate::icon::{BadIcon, Icon};

/// Object that allows you to build SystemTray instance.
pub struct SystemTrayBuilder(pub(crate) SystemTrayBuilderPlatform);

impl SystemTrayBuilder {
  /// Creates a new SystemTray for platforms where this is appropriate.
  pub fn new(icon: Icon, tray_menu: Option<ContextMenu>) -> Self {
    Self(SystemTrayBuilderPlatform::new(
      icon,
      tray_menu.map(|m| m.0.menu_platform),
    ))
  }

  /// Builds the SystemTray.
  ///
  /// Possible causes of error include denied permission, incompatible system, and lack of memory.
  pub fn build<T: 'static>(
    self,
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<SystemTray, OsError> {
    self.0.build(window_target)
  }
}

/// Represents a System Tray instance.
///
/// ## Drop behavior
///
/// **Linux:**
///   - Dropping the tray too early could lead to a default icon.
///   - Dropping the tray after the icon has been added to the system tray may not remove it.
/// **Windows:** Dropping the tray will effectively remove the icon from the system tray.
pub struct SystemTray(pub SystemTrayPlatform);

impl SystemTray {
  /// Set new tray icon.
  pub fn set_icon(&mut self, icon: Icon) {
    self.0.set_icon(icon)
  }

  /// Set new tray menu.
  pub fn set_menu(&mut self, tray_menu: &ContextMenu) {
    self.0.set_menu(&tray_menu.0.menu_platform)
  }

  /// Sets the hover text for this tray icon.
  /// 
  /// TODO: Will add support for other platforms in this PR
  #[cfg(target_os = "macos")]
  pub fn set_tool_tip(&mut self, tool_tip: &str) {
    self.0.set_tool_tip(tool_tip);
  }
}
