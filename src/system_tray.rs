// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

//! **UNSTABLE** -- The `SystemTray` struct and associated types.
//!
//! Use [SystemTrayBuilder][tray_builder] to create your tray instance.
//!
//! [ContextMenu][context_menu] is used to created a Window menu on Windows and Linux. On macOS it's used in the menubar.
//!
//! ```rust,ignore
//! let mut tray_menu = ContextMenu::new();
//! let icon = include_bytes!("my_icon.png").to_vec();
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
/// Object that allows you to build SystemTray instance.
pub struct SystemTrayBuilder(pub(crate) SystemTrayBuilderPlatform);

#[cfg(target_os = "linux")]
use std::path::PathBuf;

impl SystemTrayBuilder {
  /// Creates a new SystemTray for platforms where this is appropriate.
  ///
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[cfg(not(target_os = "linux"))]
  pub fn new(icon: Vec<u8>, tray_menu: Option<ContextMenu>) -> Self {
    Self(SystemTrayBuilderPlatform::new(
      icon,
      tray_menu.map(|m| m.0.menu_platform),
    ))
  }

  /// Creates a new SystemTray for platforms where this is appropriate.
  ///
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[cfg(target_os = "linux")]
  pub fn new(icon: PathBuf, tray_menu: Option<ContextMenu>) -> Self {
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
pub struct SystemTray(pub SystemTrayPlatform);

impl SystemTray {
  /// Set new tray icon.
  ///
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[cfg(not(target_os = "linux"))]
  pub fn set_icon(&mut self, icon: Vec<u8>) {
    self.0.set_icon(icon)
  }

  /// Set new tray icon.
  ///
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[cfg(target_os = "linux")]
  pub fn set_icon(&mut self, icon: PathBuf) {
    self.0.set_icon(icon)
  }

  /// Set new tray menu.
  pub fn set_menu(&mut self, tray_menu: &ContextMenu) {
    self.0.set_menu(&tray_menu.0.menu_platform)
  }
}
