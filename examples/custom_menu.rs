// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use simple_logger::SimpleLogger;
use tao::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  menu::{MenuBuilder, MenuType, SystemMenu},
  window::WindowBuilder,
};

fn main() {
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();

  // allocate our tray as it'll contain children
  let mut menu_bar_menu = MenuBuilder::init();

  // create our first menu
  let mut my_app_menu = MenuBuilder::init_with_title("My app");

  // create a submenu with 1 item
  let mut my_sub_menu = MenuBuilder::init();
  let (test_id, mut test_menu_item) = my_sub_menu.add_item(
    MenuType::Menubar,
    "Disable menu",
    Some("<Ctrl>d"),
    true,
    false,
  );

  // add Copy to `My App` menu
  my_app_menu.add_system_item(SystemMenu::Copy, MenuType::Menubar);

  // add our submenu under Copy
  my_app_menu.add_children(my_sub_menu, "Sub menu", true);

  let mut test_menu = MenuBuilder::init_with_title("Other menu");
  test_menu.add_item(MenuType::Menubar, "Selected and disabled", None, false, true);

  // add all our childs to menu_bar_menu (order is how they'll appear)
  menu_bar_menu.add_children(my_app_menu, "My app", true);
  menu_bar_menu.add_children(test_menu, "Other menu", true);

  let window = WindowBuilder::new()
    .with_title("A fantastic window!")
    .with_menu(menu_bar_menu)
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
        if menu_id == test_id {
          println!("Clicked on custom change menu");

          // this allow us to get access to the menu and make changes
          // without re-rendering the whole menu
          test_menu_item.disable();
        }

        println!("Clicked on {:?}", menu_id);
        window.set_title("New window title!");
      }
      _ => (),
    }
  });
}
