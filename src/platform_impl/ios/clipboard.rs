#[derive(Debug, Clone, Default)]
pub struct Clipboard;
impl Clipboard {
  pub fn write_text(&mut self, s: impl AsRef<str>) {}
  pub fn read_text(&self) -> Option<String> {}
}
