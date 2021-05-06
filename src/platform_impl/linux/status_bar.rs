use crate::{
  error::OsError, platform_impl::EventLoopWindowTarget, status_bar::Statusbar as RootStatusbar,
};

pub struct Statusbar {}
impl Statusbar {
  pub fn initialize<T>(
    _window_target: &EventLoopWindowTarget<T>,
    _status_bar: &RootStatusbar,
  ) -> Result<(), OsError> {
    debug!("`Statusbar` is ignored on Linux")
  }
}
