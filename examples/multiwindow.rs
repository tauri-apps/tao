// Copyright 2014-2021 The winit contributors
// Copyright 2021-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use tao::{
  event::{ElementState, Event, KeyEvent, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  menu::*,
  platform::macos::{WindowBuilderExtMacOS, WindowExtMacOS},
  window::{Window, WindowBuilder},
};

fn main() {
  env_logger::init();
  let event_loop = EventLoop::new();

  let mut windows = HashMap::new();

  for i in 0..2 {
    let window = WindowBuilder::new()
      .with_tabbing_identifier(if i % 2 == 0 { "0" } else { "1" })
      .build(&event_loop)
      .unwrap();
    let mut menu = MenuBar::new();
    menu.add_submenu("View", true, MenuBar::new());
    window.set_menu(Some(menu));

    println!("{:?}", window.tabbing_identifier());
    windows.insert(window.id(), window);
  }

  event_loop.run(move |event, event_loop, control_flow| {
    *control_flow = ControlFlow::Wait;

    if let Event::WindowEvent {
      event, window_id, ..
    } = event
    {
      match event {
        WindowEvent::CloseRequested => {
          println!("Window {:?} has received the signal to close", window_id);

          // This drops the window, causing it to close.
          windows.remove(&window_id);

          if windows.is_empty() {
            *control_flow = ControlFlow::Exit;
          }
        }
        WindowEvent::KeyboardInput {
          event: KeyEvent {
            state: ElementState::Pressed,
            ..
          },
          ..
        } => {
          let window = WindowBuilder::new()
            .with_tabbing_identifier("1")
            .build(event_loop)
            .unwrap();
          //window.set_tabbing_identifier("TaoWindow-TaoWindowDelegate-(null)-VT-1");
          println!("{:?}", window.tabbing_identifier());
          windows.insert(window.id(), window);
        }
        _ => (),
      }
    }
  })
}
