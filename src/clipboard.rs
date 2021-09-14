// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

//! The `Clipboard` struct and associated types.
//!
//! ## Platform-specific
//!
//! - **Android / iOS:** Unsupported
//!
//! ```rust,ignore
//! let mut cliboard = Clipboard::new();
//! cliboard.write_text("This is injected from tao!!!")
//! let content = cliboard.read_text();
//! ```
//!

use crate::platform_impl::Clipboard as ClipboardPlatform;

#[derive(Debug, Clone, Default)]
/// Object that allows you to access the `Clipboard` instance.
pub struct Clipboard(ClipboardPlatform);

impl Clipboard {
  /// Creates a new `Clipboard` instance.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  pub fn new() -> Self {
    Self::default()
  }

  /// Writes the text into the clipboard as plain text.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  pub fn write_text(&mut self, s: impl AsRef<str>) {
    self.0.write_text(s);
  }

  /// The content in the clipboard as plain text.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  pub fn read_text(&self) -> Option<String> {
    self.0.read_text()
  }
}

/// Identifier of a clipboard format.
pub(crate) type FormatId = &'static str;

/// Object that allows you to access the `ClipboardFormat`.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct ClipboardFormat {
  pub(crate) identifier: FormatId,
  pub(crate) data: Vec<u8>,
}

// todo add more formats
impl ClipboardFormat {
  #[cfg(any(target_os = "macos", target_os = "ios"))]
  pub const TEXT: &'static str = "public.utf8-plain-text";
  #[cfg(any(target_os = "windows", target_os = "android"))]
  pub const TEXT: &'static str = "text/plain";
  #[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
  ))]
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
