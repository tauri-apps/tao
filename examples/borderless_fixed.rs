// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

//! Undecorated (borderless) 500×600 window with a gray client background — no system title bar
//! and not resizable. Drag with the left mouse button to move the window.

use tao::{
  dpi::LogicalSize,
  event::{ElementState, Event, MouseButton, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

#[allow(clippy::single_match)]
fn main() {
  env_logger::init();
  let event_loop = EventLoop::new();

  let window = WindowBuilder::new()
    .with_title("Borderless + fixed size — drag to move")
    .with_inner_size(LogicalSize::new(500.0, 600.0))
    .with_background_color((192, 192, 192, 255))
    .with_decorations(false)
    .with_resizable(false)
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent { event, .. } => match event {
        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
        WindowEvent::MouseInput {
          state: ElementState::Pressed,
          button: MouseButton::Left,
          ..
        } => {
          let _ = window.drag_window();
        }
        _ => (),
      },
      _ => (),
    };
  });
}
