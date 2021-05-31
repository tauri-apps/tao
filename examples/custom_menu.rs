// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use simple_logger::SimpleLogger;
use tao::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  menu::{MenuItem, MenuType, Menubar as Menu},
  platform::macos::NativeImage,
  window::WindowBuilder,
};

fn main() {
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();

  // allocate our tray as it'll contain children
  let mut menu_bar_menu = Menu::new();

  // create our first menu
  let mut my_app_menu = Menu::new();

  // create a submenu with 1 item
  let mut my_sub_menu = Menu::new();
  let (test_id, mut test_menu_item) =
    my_sub_menu.add_custom_item("Disable menu", Some("<Ctrl>d"), true, false);
  // add Copy to `My App` menu
  my_app_menu.add_item(MenuItem::Copy);

  // add our submenu under Copy
  my_app_menu.add_submenu("Sub menu", true, my_sub_menu);

  // create another menu
  // in macOS menu bar need to be created with `init_with_title`
  let mut test_menu = Menu::new();
  test_menu.add_custom_item("Selected and disabled", None, false, true);
  test_menu.add_item(MenuItem::Separator);
  test_menu.add_custom_item("Test", None, true, false);

  // add all our childs to menu_bar_menu (order is how they'll appear)
  menu_bar_menu.add_submenu("My app", true, my_app_menu);
  menu_bar_menu.add_submenu("Other menu", true, test_menu);

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
          test_menu_item.set_enabled(false);
          test_menu_item.set_title("Menu disabled");
          test_menu_item.set_selected(true);
          test_menu_item.set_icon(NativeImage::StatusUnavailable);
        }
        println!("Clicked on {:?}", menu_id);
        window.set_title("New window title!");
      }
      _ => (),
    }
  });
}
