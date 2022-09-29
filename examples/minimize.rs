// Copyright 2014-2021 The winit contributors
// Copyright 2021-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

extern crate tao;

use tao::{
  event::{ElementState, Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  keyboard::Key,
  window::WindowBuilder,
};

#[allow(clippy::single_match)]
fn main() {
  env_logger::init();
  let event_loop = EventLoop::new();

  let window = WindowBuilder::new()
    .with_title("Hit `m` to minimize, `space` to toggle minimizability")
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent {
        event, window_id, ..
      } if window_id == window.id() => match event {
        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

        // Keyboard input event to handle minimize via a hotkey
        WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
          match event.logical_key {
            // Pressing the 'm' key will minimize the window
            // WARNING: Consider using `key_without_modifers()` if available on your platform.
            // See the `key_binding` example
            Key::Character("m") => window.set_minimized(true),
            Key::Space => {
              let minimizable = !window.is_minimizable();
              println!("Minimizable: {}", minimizable);
              window.set_minimizable(minimizable);
            }
            _ => (),
          }
        }
        _ => (),
      },
      _ => (),
    }
  });
}
