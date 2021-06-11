// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone, Default)]
pub struct Clipboard;
impl Clipboard {
  pub fn write_text(&mut self, s: impl AsRef<str>) {}
  pub fn read_text(&self) -> Option<String> {
    None
  }
}
