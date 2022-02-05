// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

fn main() {
  use tao::{
    accelerator::{Accelerator, RawMods},
    dpi::LogicalSize,
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, KeyCode, ModifiersState},
    window::WindowBuilder,
  };

  env_logger::init();

  // create a sample hotkey
  let hotkey = Accelerator::new(RawMods::Shift, KeyCode::Digit1);
  // create local modifier state
  let mut modifiers = ModifiersState::default();

  let event_loop = EventLoop::new();

  let _window = WindowBuilder::new()
    .with_inner_size(LogicalSize::new(400.0, 200.0))
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    if let Event::WindowEvent { event, .. } = event {
      match event {
        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
        WindowEvent::ModifiersChanged(new_state) => {
          // update our local modifier state
          modifiers = new_state;
        }
        // catch only pressed event
        WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
          if hotkey.matches(&modifiers, &event.physical_key) {
            println!(
              "KeyEvent:  `Shift` + `1` | logical_key: {:?}",
              &event.logical_key
            );
            // we can match manually without `Accelerator`
          } else if event.key_without_modifiers() == Key::Character("1") && modifiers.is_empty() {
            println!("KeyEvent: `1`");
          }
        }
        _ => (),
      }
    }
  });
}
