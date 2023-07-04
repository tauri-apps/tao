// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use tao::{
  dpi::LogicalUnit,
  event::{ElementState, Event, KeyEvent, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  keyboard::Key,
  window::WindowBuilder,
};

#[allow(clippy::single_match)]
fn main() {
  env_logger::init();
  let event_loop = EventLoop::new();

  let min_width = 400.0;
  let mut min_width_set = false;
  let max_width = 800.0;
  let mut max_width_set = false;
  let min_height = 200.0;
  let mut min_height_set = false;
  let max_height = 400.0;
  let mut max_height_set = false;

  let window = WindowBuilder::new().build(&event_loop).unwrap();

  eprintln!("constraint keys:");
  eprintln!("  (E) Toggle the min width");
  eprintln!("  (F) Toggle the max width");
  eprintln!("  (P) Toggle the min height");
  eprintln!("  (V) Toggle the max height");

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => *control_flow = ControlFlow::Exit,

      Event::WindowEvent {
        event:
          WindowEvent::KeyboardInput {
            event:
              KeyEvent {
                logical_key: Key::Character(key_str),
                state: ElementState::Released,
                ..
              },
            ..
          },
        ..
      } => match key_str {
        "e" => {
          min_width_set = !min_width_set;
          let min_width: Option<LogicalUnit<f64>> = min_width_set.then_some(min_width.into());
          window.set_min_inner_width(min_width);
        }
        "f" => {
          max_width_set = !max_width_set;
          let max_width: Option<LogicalUnit<f64>> = max_width_set.then_some(max_width.into());
          window.set_max_inner_width(max_width);
        }
        "p" => {
          min_height_set = !min_height_set;
          let min_height: Option<LogicalUnit<f64>> = min_height_set.then_some(min_height.into());
          window.set_min_inner_height(min_height);
        }
        "v" => {
          max_height_set = !max_height_set;
          let max_height: Option<LogicalUnit<f64>> = max_height_set.then_some(max_height.into());
          window.set_max_inner_height(max_height);
        }
        _ => {}
      },
      _ => (),
    }
  });
}
