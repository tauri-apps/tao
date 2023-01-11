use gdk::Display;

use crate::{dpi::PhysicalPosition, error::ExternalError};

#[inline]
pub fn cursor_position() -> Result<PhysicalPosition<f64>, ExternalError> {
  Display::default()
    .and_then(|d| d.default_seat())
    .and_then(|s| s.pointer())
    .map(|p| p.position_double())
    .map(|(_, x, y)| (x, y).into())
    .ok_or(ExternalError::Os(os_error!(super::OsError)))
}
