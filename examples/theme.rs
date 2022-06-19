// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[allow(clippy::single_match)]
fn main() {
  use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
  };

  #[cfg(any(target_os = "macos"))]
  use tao::platform::macos::{WindowBuilderExtMacOS, WindowExtMacOS};
  #[cfg(target_os = "windows")]
  use tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};

  env_logger::init();
  let event_loop = EventLoop::new();

  let window = WindowBuilder::new()
    .with_title("A fantastic window!")
    // .with_theme(Some(tao::window::Theme::Light))
    .build(&event_loop)
    .unwrap();

  println!("Initial theme: {:?}", window.theme());

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => *control_flow = ControlFlow::Exit,
      Event::WindowEvent {
        event: WindowEvent::ThemeChanged(theme),
        window_id,
        ..
      } if window_id == window.id() => {
        println!("Theme is changed: {:?}", theme)
      }
      _ => (),
    }
  });
}

#[cfg(any(
  target_os = "ios",
  target_os = "android",
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]
fn main() {
  println!("This platform doesn't support theme.");
}
