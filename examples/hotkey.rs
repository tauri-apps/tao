// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use simple_logger::SimpleLogger;
use tao::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  keyboard::Hotkey,
  menu::{Menu, MenuItem, MenuType},
  window::WindowBuilder,
};

fn main() {
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();

  let hotkey = Hotkey::new("Ctrl+,+j").expect("Invalid hotkey");

  let test_menu =
    MenuItem::new("Test menu").with_accelerators(hotkey);

  let test_menu_id = test_menu.id();

  let window = WindowBuilder::new()
    .with_title("A fantastic window!")
    .with_menu(vec![
      Menu::new(
        // on macOS first menu is always app name
        "my app",
        vec![
          test_menu,
          MenuItem::Separator,
          MenuItem::CloseWindow,
        ],
      ),
    ])
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        window_id,
      } if window_id == window.id() => *control_flow = ControlFlow::Exit,
      Event::MainEventsCleared => {
        window.request_redraw();
      }
      Event::WindowEvent{
        event: WindowEvent::KeyboardInput{
          device_id,
          input,
          is_synthetic
        },
        window_id,
      } => {
        println!("input {:?} {:?} {} {:?}", device_id, input, is_synthetic, window_id);
      }
      Event::MenuEvent {
        menu_id,
        origin: MenuType::Menubar,
      } => {
        if menu_id == test_menu_id {
          println!("Clicked on test menu");
        }
        println!("Clicked on {:?}", menu_id);
        window.set_title("New window title!");
      }
      _ => (),
    }
  });
}
