// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use simple_logger::SimpleLogger;
#[cfg(target_os = "macos")]
use tao::platform::hotkey::HotKeyExtGlobalAccelerator;
use tao::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  hotkey::{HotKey, RawMods},
  keyboard::Key,
  window::WindowBuilder,
};

fn main() {
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();

  let window = WindowBuilder::new()
    .with_title("A fantastic window!")
    .build(&event_loop)
    .unwrap();

  // register global hotkey without modifier
  let _global_hotkey_1 = HotKey::new(None, Key::F13).register_global();

  // register global hotkey with combined modifier + b
  // Command + Alt + Shift on macOS, Ctrl + Alt + Shift on windows/linux
  let _global_hotkey_2 = HotKey::new(RawMods::AltCtrlMeta, "b").register_global();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        window_id,
      } if window_id == window.id() => *control_flow = ControlFlow::Exit,
      Event::MainEventsCleared => {
        window.request_redraw();
      }
      _ => (),
    }
  });
}
