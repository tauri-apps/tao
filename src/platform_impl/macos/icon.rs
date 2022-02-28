use crate::icon::BadIcon;

pub struct Icon {
  pub(crate) rgba: Vec<u8>,
  pub(crate) width: i32,
  pub(crate) height: i32,
}

impl Icon {
  pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, BadIcon> {
    Ok(Self {
      rgba,
      width,
      height,
    })
  }
}
