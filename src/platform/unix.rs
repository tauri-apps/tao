#![cfg(any(
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

use crate::{
  error::OsError, event_loop::EventLoopWindowTarget, menu::MenuItem, platform_impl, window::Window,
};
use std::path::PathBuf;

pub use crate::platform_impl::hit_test;

/// Additional methods on `Window` that are specific to Unix.
pub trait WindowExtUnix {
  /// Returns the `ApplicatonWindow` from gtk crate that is used by this window.
  fn gtk_window(&self) -> &gtk::ApplicationWindow;

  /// Not to display window icon in the task bar.
  fn skip_taskbar(&self);
}

impl WindowExtUnix for Window {
  fn gtk_window(&self) -> &gtk::ApplicationWindow {
    &self.window.window
  }

  fn skip_taskbar(&self) {
    self.window.skip_taskbar()
  }
}

/// Status bar is a system tray icon usually display on top right or bottom right of the screen.
#[derive(Debug, Clone)]
#[cfg(target_os = "linux")]
pub struct Statusbar {
  pub(crate) icon: PathBuf,
  pub(crate) items: Vec<MenuItem>,
}

pub struct StatusbarBuilder {
  status_bar: Statusbar,
}

impl StatusbarBuilder {
  /// Creates a new Statusbar for platforms where this is appropriate.
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
