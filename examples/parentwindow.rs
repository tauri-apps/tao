// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

fn main() {
  use std::collections::HashMap;
  use tao::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
  };
  env_logger::init();
  let event_loop = EventLoop::new();
  let mut windows = HashMap::new();
  let main_window = WindowBuilder::new().build(&event_loop).unwrap();

  let child_window = WindowBuilder::new()
    .with_parent_window(&main_window)
    .with_inner_size(LogicalSize::new(200, 200))
    .build(&event_loop)
    .unwrap();

  windows.insert(child_window.id(), child_window);
  windows.insert(main_window.id(), main_window);

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::NewEvents(StartCause::Init) => println!("TAO application started!"),
      Event::WindowEvent {
        event, window_id, ..
      } if event == WindowEvent::CloseRequested => {
        println!("Window {:?} has received the signal to close", window_id);
        // This drop the window, causing it to close.
        windows.remove(&window_id);
        if windows.is_empty() {
          *control_flow = ControlFlow::Exit;
        }
      }
      _ => (),
    };
  })
}
