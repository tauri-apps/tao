use crate::icon::{BadIcon, RgbaIcon};
use std::io::BufWriter;

#[derive(Debug, Clone)]
pub struct PlatformIcon(RgbaIcon);

impl PlatformIcon {
  pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, BadIcon> {
    Ok(PlatformIcon(RgbaIcon::from_rgba(rgba, width, height)?))
  }

  pub fn to_png(&self) -> Vec<u8> {
    let png = Vec::new();
    let ref mut w = BufWriter::new(png);

    let mut encoder = png::Encoder::new(w, self.0.width as _, self.0.height as _);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&self.0.rgba).unwrap();

    png
  }
}
