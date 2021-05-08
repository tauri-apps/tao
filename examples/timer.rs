// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use instant::Instant;
use std::time::Duration;

use simple_logger::SimpleLogger;
use tao::{
  event::{Event, StartCause, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

#[allow(clippy::single_match)]
fn main() {
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();

  let _window = WindowBuilder::new()
    .with_title("A fantastic window!")
    .build(&event_loop)
    .unwrap();

  let timer_length = Duration::new(1, 0);

  event_loop.run(move |event, _, control_flow| {
    println!("{:?}", event);

    match event {
      Event::NewEvents(StartCause::Init) => {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + timer_length)
      }
      Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + timer_length);
        println!("\nTimer\n");
      }
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => *control_flow = ControlFlow::Exit,
      _ => (),
    }
  });
}
