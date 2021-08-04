// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn main() {
  #[cfg(target_os = "macos")]
  use tao::platform::macos::{WindowBuilderExtMacOS, WindowExtMacOS};
  #[cfg(target_os = "windows")]
  use tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};
  #[cfg(target_os = "windows")]
  use winapi::shared::windef::HWND;
  use simple_logger::SimpleLogger;
  use std::collections::HashMap;
  use tao::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
  };
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();
  let mut windows = HashMap::new();
  let main_window = WindowBuilder::new().build(&event_loop).unwrap();

  #[cfg(target_os = "macos")]
  let parent_window = main_window.ns_window();
  #[cfg(target_os = "windows")]
  let parent_window = main_window.hwnd() as HWND;

  windows.insert(main_window.id(), main_window);

  event_loop.run(move |event, event_loop, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::NewEvents(StartCause::Init) => {
        println!("TAO application started!");
        let child_window = WindowBuilder::new()
          .with_parent_window(parent_window)
          .with_inner_size(LogicalSize::new(200, 200))
          .build(event_loop)
          .unwrap();
        windows.insert(child_window.id(), child_window);
      }
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

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn main() {
  println!("This platform doesn't have the parent window support.");
}
