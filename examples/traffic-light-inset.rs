// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use tao::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  platform::macos::{WindowBuilderExtMacOS, WindowExtMacOS},
  window::WindowBuilder, dpi::LogicalSize,
};

#[allow(clippy::single_match)]
fn main() {
  let event_loop = EventLoop::new();

  let mut window = Some(
    WindowBuilder::new()
      .with_title("A fantastic window!")
      .with_inner_size(tao::dpi::LogicalSize::new(128.0, 128.0))
      .with_traffic_light_inset(LogicalSize {  width: 32., height: 32. })
      .with_resizable(true)
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
      Event::RedrawRequested(_) => {
        if let Some(w) = &window {
          // set the traffic light inset after each redraw to prevent visual artifacts
          w.set_traffic_light_inset(LogicalSize {  width: 32., height: 32. });
          w.request_redraw();
        }
      }
      _ => (),
    }
  });
}