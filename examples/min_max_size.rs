// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use tao::{
  dpi::LogicalSize,
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

#[allow(clippy::single_match)]
fn main() {
  env_logger::init();
  let event_loop = EventLoop::new();

  let window = WindowBuilder::new().build(&event_loop).unwrap();

  window.set_min_inner_size(Some(LogicalSize::new(400.0, 200.0)));
  window.set_max_inner_size(Some(LogicalSize::new(800.0, 400.0)));

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;
    println!("{:?}", event);

    match event {
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => *control_flow = ControlFlow::Exit,
      _ => (),
    }
  });
}
