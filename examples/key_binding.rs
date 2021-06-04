// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
fn main() {
  use simple_logger::SimpleLogger;
  use tao::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState},
    window::WindowBuilder,
  };

  use tao::platform::modifier_supplement::KeyEventExtModifierSupplement;

  SimpleLogger::new().init().unwrap();

  fn handle_key_event(modifiers: ModifiersState, event: KeyEvent) {
    if event.state == ElementState::Pressed
      && !event.repeat
      // catch only 1 without the modifier, we'll match later...
      && event.key_without_modifiers() == Key::Character("1")
    {
      if modifiers.shift_key() {
        println!("Shift + 1 | logical_key: {:?}", event.logical_key);
      } else {
        println!("1");
      }
    }
  }

  let event_loop = EventLoop::new();

  let _window = WindowBuilder::new()
    .with_inner_size(LogicalSize::new(400.0, 200.0))
    .build(&event_loop)
    .unwrap();

  let mut modifiers = ModifiersState::default();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    if let Event::WindowEvent { event, .. } = event {
      match event {
        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
        WindowEvent::ModifiersChanged(new_state) => {
          modifiers = new_state;
        }
        WindowEvent::KeyboardInput { event, .. } => {
          handle_key_event(modifiers, event);
        }
        _ => (),
      }
    }
  });
}

// System tray isn't supported on other's platforms.
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn main() {
  println!("This platform doesn't support `KeyEventExtModifierSupplement`.");
}
