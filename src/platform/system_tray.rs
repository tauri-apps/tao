// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(any(
  target_os = "windows",
  target_os = "macos",
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

use crate::{error::OsError, event_loop::EventLoopWindowTarget, menu::MenuItem, platform_impl};
//TODO exhaustively match the targets
#[cfg(target_os = "linux")]
use std::path::PathBuf;

/// System tray is a status icon that can show popup menu. It is usually displayed on top right or bottom right of the screen.
///
/// ## Platform-specific
///
/// - **Linux:**: require `menu` feature flag. Otherwise, it's a no-op.
#[derive(Debug)]
pub struct SystemTray {
  #[cfg(target_os = "linux")]
  pub(crate) icon: PathBuf,
  #[cfg(not(target_os = "linux"))]
  pub(crate) icon: Vec<u8>,
  pub(crate) items: Vec<MenuItem>,
  instance: platform_impl::SystemTray,
}

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
  #[cfg(not(target_os = "linux"))]
  pub fn new(icon: Vec<u8>, items: Vec<MenuItem>) -> Self {
    Self {
      system_tray: SystemTray { icon, items },
    }
  }

  /// Creates a new SystemTray for platforms where this is appropriate.
  #[inline]
  #[cfg(target_os = "linux")]
  pub fn new(icon: PathBuf, items: Vec<MenuItem>) -> Self {
    Self {
      system_tray: SystemTray {
        icon,
        items,
        instance: platform_impl::SystemTray::new(),
      },
    }
  }

  /// Builds the system tray.
  ///
  /// Possible causes of error include denied permission, incompatible system, and lack of memory.
  #[inline]
  pub fn build<T: 'static>(
    mut self,
    _window_target: &EventLoopWindowTarget<T>,
  ) -> Result<SystemTray, OsError> {
    self.system_tray.instance.initialize(
      &_window_target.p,
      &self.system_tray.icon,
      &self.system_tray.items,
    )?;
    Ok(self.system_tray)
  }

  /// Allows changing the icon during runtime.
  #[inline]
  #[cfg(target_os = "linux")]
  pub fn set_icon(&mut self, icon: &PathBuf) -> Result<(), OsError> {
    self.system_tray.instance.set_icon(&icon)?;
    Ok(())
  }
}
