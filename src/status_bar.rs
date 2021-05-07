use crate::{error::OsError, event_loop::EventLoopWindowTarget, menu::MenuItem, platform_impl};
use std::path::PathBuf;

/// Status bar is a system tray icon usually display on top right or bottom right of the screen.
///
/// ## Platform-specific
///
/// - **Android / iOS:** Unsupported
#[derive(Debug, Clone)]
pub struct Statusbar {
  pub(crate) icon: PathBuf,
  pub(crate) items: Vec<MenuItem>,
}

pub struct StatusbarBuilder {
  status_bar: Statusbar,
}

impl StatusbarBuilder {
  /// Creates a new Statusbar for platforms where this is appropriate.
  ///
  /// Error should be very rare and only occur in case of permission denied, incompatible system,
  /// out of memory, invalid icon.
  #[inline]
  pub fn new(icon: PathBuf, items: Vec<MenuItem>) -> Self {
    //let icon = std::fs::read(icon)?;
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
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<Statusbar, OsError> {
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux",))]
    platform_impl::Statusbar::initialize(&window_target.p, &self.status_bar)?;
    #[cfg(any(target_os = "android", target_os = "ios"))]
    debug!("`StatusBar` is not supported on this platform");
    Ok(self.status_bar)
  }
}
