#[derive(Debug, Clone)]
pub struct Icon {
  rgba: Vec<u8>,
  width: i32,
  height: i32,
}

impl Icon {
  pub(crate) fn to_png(&self) -> Vec<u8> {
    let png = Vec::new();
    let ref mut w = BufWriter::new(png);

    let mut encoder = png::Encoder::new(w, self.width, self.height); // Width is 2 pixels and height is 1.
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&self.rgba).unwrap();

    png
  }
}
