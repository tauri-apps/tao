use crate::platform_impl::Clipboard as ClipboardPlatform;

#[derive(Debug, Clone, Default)]
pub struct Clipboard(ClipboardPlatform);

impl Clipboard {
  /// Creates a new Clipboard instance.
  pub fn new() -> Self {
   Self::default()
 }

   /// Put a string onto the system clipboard.
   pub fn put_string(&mut self, s: impl AsRef<str>) {
       self.0.put_string(s);
   }
    /// Get a string from the system clipboard, if one is available.
    pub fn get_string(&self) -> Option<String> {
      self.0.get_string()
  }
}


pub type FormatId = &'static str;

#[derive(Debug, Clone)]
pub struct ClipboardFormat {
    pub(crate) identifier: FormatId,
    pub(crate) data: Vec<u8>,
}

// todo add more formats
impl ClipboardFormat {
  #[cfg(target_os = "macos")]
  pub const TEXT: &'static str = "public.utf8-plain-text";
  #[cfg(target_os = "window")]
  pub const TEXT: &'static str = "text/plain";
  #[cfg(target_os = "linux")]
  pub const TEXT: &'static str = "UTF8_STRING";
}

impl ClipboardFormat {
  pub fn new(identifier: FormatId, data: impl Into<Vec<u8>>) -> Self {
      let data = data.into();
      ClipboardFormat { identifier, data }
  }
}

impl From<String> for ClipboardFormat {
  fn from(src: String) -> ClipboardFormat {
      let data = src.into_bytes();
      ClipboardFormat::new(ClipboardFormat::TEXT, data)
  }
}

impl From<&str> for ClipboardFormat {
  fn from(src: &str) -> ClipboardFormat {
      src.to_string().into()
  }
}