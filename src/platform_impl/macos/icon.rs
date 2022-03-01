use crate::icon::BadIcon;
#[derive(Debug, Clone)]
pub struct PlatformIcon {
  pub(crate) rgba: Vec<u8>,
  pub(crate) width: u32,
  pub(crate) height: u32,
}

impl PlatformIcon {
  pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, BadIcon> {
    Ok(Self {
      rgba,
      width,
      height,
    })
  }
}
