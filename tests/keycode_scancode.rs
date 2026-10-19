// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(any(
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

use tao::keyboard::KeyCode;

#[test]
fn keycode_scancode_round_trip() {
  let codes = [
    KeyCode::BracketLeft,
    KeyCode::BracketRight,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyR,
    KeyCode::Numpad0,
    KeyCode::Numpad1,
    KeyCode::Numpad2,
    KeyCode::Numpad3,
    KeyCode::Numpad4,
    KeyCode::Numpad5,
    KeyCode::Numpad6,
    KeyCode::Numpad7,
    KeyCode::Numpad8,
    KeyCode::Numpad9,
  ];

  for code in codes {
    let scancode = code.to_scancode().unwrap();
    assert_eq!(
      KeyCode::from_scancode(scancode),
      code,
      "scancode {scancode:#x} of {code:?} maps back to a different key"
    );
  }
}
