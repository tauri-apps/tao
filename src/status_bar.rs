use crate::{error::OsError, event_loop::EventLoopWindowTarget, menu::MenuItem, platform_impl};
use std::{io, path::PathBuf};

#[derive(Debug, Clone)]
pub struct Statusbar {
  pub icon: Vec<u8>,
  pub items: Vec<MenuItem>,
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
  pub fn new(icon: PathBuf, items: Vec<MenuItem>) -> Result<Self, io::Error> {
    let icon = std::fs::read(icon)?;
    Ok(Self {
      status_bar: Statusbar { icon, items },
    })
  }

  /// Builds the status bar.
  ///
  /// Possible causes of error include denied permission, incompatible system, and lack of memory.
  #[inline]
  pub fn build<T: 'static>(
    self,
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<Statusbar, OsError> {
    platform_impl::Statusbar::initialize(&window_target.p, &self.status_bar)?;
    Ok(self.status_bar)
  }
}
