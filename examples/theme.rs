// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#[allow(clippy::single_match)]
fn main() {
  use tao::{
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Theme, WindowBuilder},
  };

  env_logger::init();
  let event_loop = EventLoop::new();

  let window = WindowBuilder::new()
    .with_title("A fantastic window!")
    // .with_theme(Some(tao::window::Theme::Light))
    .build(&event_loop)
    .unwrap();

  println!("Initial theme: {:?}", window.theme());

  let mut theme = true;

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent {
        event:
          WindowEvent::MouseInput {
            button: MouseButton::Left,
            state: ElementState::Pressed,
            ..
          },
        ..
      } => {
        theme = !theme;
        window.set_theme(if theme { None } else { Some(Theme::Dark) })
      }
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => *control_flow = ControlFlow::Exit,
      Event::WindowEvent {
        event: WindowEvent::ThemeChanged(theme),
        window_id,
        ..
      } if window_id == window.id() => {
        println!("Theme is changed: {theme:?}")
      }
      _ => (),
    }
  });
}
