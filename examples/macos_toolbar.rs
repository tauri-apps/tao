// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// This example heavily uses the window.rs example as a template

use tao::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  platform::macos::WindowBuilderExtMacOS,
  window::WindowBuilder,
};

#[allow(clippy::single_match)]
fn main() {
  let event_loop = EventLoop::new();

  let mut window = Some(
    WindowBuilder::new()
      .with_title("xxx") // Set title to "" to hide the title without making the toolbar slimmer
      .with_inner_size(tao::dpi::LogicalSize::new(128.0, 128.0))
      .with_toolbar(true)
      // .with_title_hidden(true) hides the title but makes the toolbar slimmer
      .build(&event_loop)
      .unwrap(),
  );

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        window_id: _,
        ..
      } => {
        // drop the window to fire the `Destroyed` event
        window = None;
      }
      Event::WindowEvent {
        event: WindowEvent::Destroyed,
        window_id: _,
        ..
      } => {
        *control_flow = ControlFlow::Exit;
      }
      Event::MainEventsCleared => {
        if let Some(w) = &window {
          w.request_redraw();
        }
      }
      _ => (),
    }
  });
}
