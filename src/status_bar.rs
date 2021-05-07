pub use crate::platform_impl::Statusbar;
use crate::{error::OsError, event_loop::EventLoopWindowTarget, menu::MenuItem};
#[cfg(target_os = "linux")]
use std::path::PathBuf;

pub struct StatusbarBuilder {
  status_bar: Statusbar,
}

impl StatusbarBuilder {
  /// Creates a new Statusbar for platforms where this is appropriate.
  ///
  /// Error should be very rare and only occur in case of permission denied, incompatible system,
  /// out of memory, invalid icon.
  #[inline]
  #[cfg(not(target_os = "linux"))]
  pub fn new(icon: Vec<u8>, items: Vec<MenuItem>) -> Self {
    Self {
      status_bar: Statusbar { icon, items },
    }
  }

  /// Creates a new Statusbar for platforms where this is appropriate.
  ///
  /// Error should be very rare and only occur in case of permission denied, incompatible system,
  /// out of memory, invalid icon.
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
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<Statusbar, OsError> {
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux",))]
    Statusbar::initialize(&window_target.p, &self.status_bar)?;
    #[cfg(any(target_os = "android", target_os = "ios"))]
    debug!("`StatusBar` is not supported on this platform");
    Ok(self.status_bar)
  }
}
