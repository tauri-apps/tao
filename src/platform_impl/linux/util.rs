use gdk::Display;

use crate::{
  dpi::{LogicalPosition, PhysicalPosition},
  error::ExternalError,
};
use std::process::{Command, Stdio};

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

pub fn is_unity() -> bool {
  if let Ok(child) = Command::new("echo")
    .arg("$XDG_CURRENT_DESKTOP")
    .stdout(Stdio::piped())
    .spawn()
  {
    if let Ok(output) = child.wait_with_output() {
      if let Ok(string) = String::from_utf8(output.stdout) {
        let string = string.as_str().to_lowercase();
        string == "unity" || string == "gnome"
      } else {
        false
      }
    } else {
      false
    }
  } else {
    false
  }
}
