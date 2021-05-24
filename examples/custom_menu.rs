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

  let hotkey = Hotkey::new("F1").unwrap();

  println!("hotkey {:?}", hotkey);

  let custom_change_menu =
    MenuItem::new("Change menu").with_accelerators(Hotkey::new("F1").unwrap());
  let custom_change_menu_id = custom_change_menu.id();

  let window = WindowBuilder::new()
    .with_title("A fantastic window!")
    .with_menu(vec![
      Menu::new(
        // on macOS first menu is always app name
        "my custom app",
        vec![
          MenuItem::About("Todos".to_string()),
          MenuItem::Services,
          MenuItem::Separator,
          MenuItem::Hide,
          MenuItem::HideOthers,
          MenuItem::ShowAll,
          MenuItem::Separator,
          MenuItem::Quit,
        ],
      ),
      Menu::new(
        "File",
        vec![
          custom_change_menu,
          MenuItem::Separator,
          MenuItem::CloseWindow,
        ],
      ),
      Menu::new(
        "Edit",
        vec![
          MenuItem::Undo,
          MenuItem::Redo,
          MenuItem::Separator,
          MenuItem::Cut,
          MenuItem::Copy,
          MenuItem::Paste,
          MenuItem::Separator,
          MenuItem::SelectAll,
        ],
      ),
      Menu::new("View", vec![MenuItem::EnterFullScreen]),
      Menu::new("Window", vec![MenuItem::Minimize, MenuItem::Zoom]),
      Menu::new(
        "Help",
        vec![MenuItem::new("Custom help").with_accelerators(Hotkey::new("CTRL+SHIFT+h").unwrap())],
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
      Event::MenuEvent {
        menu_id,
        origin: MenuType::Menubar,
      } => {
        if menu_id == custom_change_menu_id {
          println!("Clicked on custom change menu");
          window.set_menu(Some(vec![Menu::new(
            "File",
            vec![
              MenuItem::new("Add Todo").with_accelerators(Hotkey::new("CTRL+a").unwrap()),
              MenuItem::Separator,
              MenuItem::CloseWindow,
            ],
          )]))
        }

        println!("Clicked on {:?}", menu_id);
        window.set_title("New window title!");
      }
      _ => (),
    }
  });
}
