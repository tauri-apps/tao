#[derive(Debug, Clone, Default)]
pub struct Clipboard;
impl Clipboard {
  pub fn put_string(&mut self, s: impl AsRef<str>) {}
  pub fn get_string(&self) -> Option<String> {}
}
