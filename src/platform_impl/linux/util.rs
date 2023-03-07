use gdk::Display;

use crate::{
  dpi::{LogicalPosition, PhysicalPosition},
  error::ExternalError,
};

#[inline]
pub fn cursor_position(is_wayland: bool) -> Result<PhysicalPosition<f64>, ExternalError> {
  if is_wayland {
    Ok((0, 0).into())
  } else {
    Display::default()
      .map(|d| {
        (
          d.default_seat().and_then(|s| s.pointer()),
          d.default_group(),
        )
      })
      .map(|(p, g)| {
        p.map(|p| {
          let (_, x, y) = p.position_double();
          LogicalPosition::new(x, y).to_physical(g.scale_factor() as _)
        })
      })
      .map(|p| p.ok_or(ExternalError::Os(os_error!(super::OsError))))
      .ok_or(ExternalError::Os(os_error!(super::OsError)))?
  }
}
