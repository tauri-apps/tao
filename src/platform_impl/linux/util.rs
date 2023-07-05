use gdk::Display;
use gtk::traits::{GtkWindowExt, WidgetExt};

use crate::{
  dpi::{LogicalPosition, PhysicalPosition, Unit},
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

pub fn set_size_constraints<W: GtkWindowExt + WidgetExt>(
  window: &W,
  min_width: Option<Unit>,
  min_height: Option<Unit>,
  max_width: Option<Unit>,
  max_height: Option<Unit>,
) {
  let mut geom_mask = gdk::WindowHints::empty();
  if min_width.is_some() || min_height.is_some() {
    geom_mask |= gdk::WindowHints::MIN_SIZE;
  }
  if max_width.is_some() || max_height.is_some() {
    geom_mask |= gdk::WindowHints::MAX_SIZE;
  }

  let scale_factor = window.scale_factor() as f64;

  let min_width = min_width
    .map(|u| u.to_logical::<f64>(scale_factor).0 as i32)
    .unwrap_or_default();
  let min_height = min_height
    .map(|u| u.to_logical::<f64>(scale_factor).0 as i32)
    .unwrap_or_default();
  let max_width = max_width
    .map(|u| u.to_logical::<f64>(scale_factor).0 as i32)
    .unwrap_or(i32::MAX);
  let max_height = max_height
    .map(|u| u.to_logical::<f64>(scale_factor).0 as i32)
    .unwrap_or(i32::MAX);

  let picky_none: Option<&gtk::Window> = None;
  window.set_geometry_hints(
    picky_none,
    Some(&gdk::Geometry::new(
      min_width,
      min_height,
      max_width,
      max_height,
      0,
      0,
      0,
      0,
      0f64,
      0f64,
      gdk::Gravity::Center,
    )),
    geom_mask,
  )
}
