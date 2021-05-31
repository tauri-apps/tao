// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(
  target_os = "macos",
  target_os = "windows",
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

fn main() {
  use simple_logger::SimpleLogger;
  use std::collections::HashMap;
  #[cfg(target_os = "linux")]
  use std::path::Path;
  #[cfg(target_os = "macos")]
  use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
  use tao::{
    dpi::LogicalSize,
    event::{Event, Rectangle, TrayEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::system_tray::SystemTrayBuilder,
    window::WindowBuilder,
  };

  SimpleLogger::new().init().unwrap();
  #[cfg(target_os = "macos")]
  let mut event_loop = EventLoop::new();

  #[cfg(not(target_os = "macos"))]
  let event_loop = EventLoop::new();

  // launch macos app without menu and without dock icon
  // shouold be set at launch
  #[cfg(target_os = "macos")]
  event_loop.set_activation_policy(ActivationPolicy::Accessory);

  let mut windows = HashMap::new();

  fn window_position_center_tray(rectangle: &mut Rectangle, window_width: f64) -> Rectangle {
    let window_x = rectangle.position.x + ((rectangle.size.width / 2.0) - (window_width / 2.0));
    rectangle.position.x = window_x;
    *rectangle
  }

  // Windows require Vec<u8> ICO file
  #[cfg(target_os = "windows")]
  let icon = include_bytes!("icon.ico").to_vec();
  // macOS require Vec<u8> PNG file
  #[cfg(target_os = "macos")]
  let icon = include_bytes!("icon.png").to_vec();
  // Linux require Pathbuf to PNG file
  #[cfg(target_os = "linux")]
  let icon = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/icon.png");

  // Only supported on macOS, linux and windows
  #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
  let _system_tray = SystemTrayBuilder::new(icon, None)
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, event_loop, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent { event, window_id } => {
        if event == WindowEvent::CloseRequested {
          println!("Window {:?} has received the signal to close", window_id);
          // Remove window from our hashmap
          windows.remove(&window_id);
        }
      }
      // NOTE: tray event's are always sent, even if menu is set
      Event::TrayEvent { mut bounds, event } => {
        if event == TrayEvent::LeftClick {
          println!("{:?}", bounds.position);
          if windows.is_empty() {
            // window size
            let window_inner_size = LogicalSize::new(200.0, 200.0);
            // create our window
            let window = WindowBuilder::new()
              // position our window centered with tray bounds
              .with_position(
                window_position_center_tray(&mut bounds, window_inner_size.width).position,
              )
              .with_inner_size(window_inner_size)
              .with_resizable(false)
              .build(&event_loop)
              .unwrap();
            windows.insert(window.id(), window);
          } else {
            for window in windows.values() {
              window.set_focus();
            }
          }
        }
      }

      _ => (),
    }
  });
}

#[cfg(any(target_os = "ios", target_os = "android",))]
fn main() {
  println!("This platform doesn't support run_return.");
}
