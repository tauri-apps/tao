// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use gdk::Atom;
use gtk::{TargetEntry, TargetFlags};

#[derive(Debug, Clone)]
pub struct Clipboard;

const CLIPBOARD_TARGETS: [&str; 5] = [
  "UTF8_STRING",
  "TEXT",
  "STRING",
  "text/plain;charset=utf-8",
  "text/plain",
];

impl Clipboard {
  pub fn write_text(&mut self, string: impl AsRef<str>) {
    let string = string.as_ref().to_string();

    let display = gdk::Display::get_default().unwrap();
    let clipboard = gtk::Clipboard::get_default(&display).unwrap();

    let targets: Vec<TargetEntry> = CLIPBOARD_TARGETS
      .iter()
      .enumerate()
      .map(|(i, target)| TargetEntry::new(target, TargetFlags::all(), i as u32))
      .collect();

    clipboard.set_with_data(&targets, move |_, selection, _| {
      selection.set(&selection.get_target(), 8 as i32, string.as_bytes());
    });
  }

  pub fn read_text(&self) -> Option<String> {
    let display = gdk::Display::get_default().unwrap();
    let clipboard = gtk::Clipboard::get_default(&display).unwrap();

    for target in &CLIPBOARD_TARGETS {
      let atom = Atom::intern(target);
      if let Some(selection) = clipboard.wait_for_contents(&atom) {
        return String::from_utf8(selection.get_data()).ok();
      }
    }

    None
  }
}
