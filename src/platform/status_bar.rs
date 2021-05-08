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

/// Status bar is a system tray icon usually display on top right or bottom right of the screen.
#[derive(Debug, Clone)]
pub struct Statusbar {
  #[cfg(target_os = "linux")]
  pub(crate) icon: PathBuf,
  #[cfg(not(target_os = "linux"))]
  pub(crate) icon: Vec<u8>,
  pub(crate) items: Vec<MenuItem>,
}

pub struct StatusbarBuilder {
  status_bar: Statusbar,
}

impl StatusbarBuilder {
  /// Creates a new Statusbar for platforms where this is appropriate.
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[inline]
  #[cfg(not(target_os = "linux"))]
  pub fn new(icon: Vec<u8>, items: Vec<MenuItem>) -> Self {
    Self {
      status_bar: Statusbar { icon, items },
    }
  }

  /// Creates a new Statusbar for platforms where this is appropriate.
  #[inline]
  #[cfg(target_os = "linux")]
  pub fn new(icon: PathBuf, items: Vec<MenuItem>) -> Self {
    Self {
      status_bar: Statusbar { icon, items },
    }
  }

  /// Builds the status bar.
  ///
  /// Possible causes of error include denied permission, incompatible system, and lack of memory.
  #[inline]
  pub fn build<T: 'static>(
    self,
    _window_target: &EventLoopWindowTarget<T>,
  ) -> Result<Statusbar, OsError> {
    platform_impl::Statusbar::initialize(&_window_target.p, &self.status_bar)?;
    Ok(self.status_bar)
  }
}
