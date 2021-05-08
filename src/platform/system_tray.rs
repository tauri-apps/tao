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
#[derive(Debug, Clone)]
pub struct SystemTray {
  #[cfg(target_os = "linux")]
  pub(crate) icon: PathBuf,
  #[cfg(not(target_os = "linux"))]
  pub(crate) icon: Vec<u8>,
  pub(crate) items: Vec<MenuItem>,
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
      system_tray: SystemTray { icon, items },
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
    platform_impl::SystemTray::initialize(&_window_target.p, &self.system_tray)?;
    Ok(self.system_tray)
  }
}
