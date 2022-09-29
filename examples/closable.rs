// Copyright 2014-2021 The winit contributors
// Copyright 2021-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use tao::{
  dpi::LogicalSize,
  event::{ElementState, Event, KeyEvent, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  keyboard::KeyCode,
  window::WindowBuilder,
};

#[allow(clippy::single_match)]
fn main() {
  env_logger::init();
  let event_loop = EventLoop::new();

  // let mut closable = false;

  let window = WindowBuilder::new()
    .with_title("Hit space to toggle closability.")
    .with_inner_size(LogicalSize::new(400.0, 200.0))
    .with_closable(false)
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent { event, .. } => match event {
        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
        WindowEvent::KeyboardInput {
          event:
            KeyEvent {
              physical_key: KeyCode::Space,
              state: ElementState::Released,
              ..
            },
          ..
        } => {
          let closable = !window.is_closable();
          println!("closable: {}", closable);
          window.set_closable(closable);
        }
        _ => (),
      },
      _ => (),
    };
  });
}
