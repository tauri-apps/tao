// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use simple_logger::SimpleLogger;
#[cfg(target_os = "macos")]
use tao::platform::macos::{CustomMenuItemExtMacOS, NativeImage};
use tao::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  menu::{MenuBar as Menu, MenuItemAttributes, MenuType, Menuitem},
  window::WindowBuilder,
};

fn main() {
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();

  // allocate our tray as it'll contain children
  let mut menu_bar_menu = Menu::new();

  // create our first menu
  let mut my_app_menu = Menu::new();

  // create a submenu
  let mut my_sub_menu = Menu::new();

  let mut test_menu_item =
    my_sub_menu.add_item(MenuItemAttributes::new("Disable menu").with_accelerators("<Primary>d"));
  // add Copy to `My App` menu
  my_app_menu.add_native_item(Menuitem::Copy);

  // add our submenu under Copy
  my_app_menu.add_submenu("Sub menu", true, my_sub_menu);

  let mut test_menu = Menu::new();
  test_menu.add_item(
    MenuItemAttributes::new("Selected and disabled")
      .with_enabled(true)
      .with_selected(true),
  );
  test_menu.add_native_item(Menuitem::Separator);
  test_menu.add_item(MenuItemAttributes::new("Test"));

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
        origin: MenuType::MenuBar,
      } if menu_id == test_menu_item.clone().id() => {
        println!("Clicked on custom change menu");
        // this allow us to get access to the menu and make changes
        // without re-rendering the whole menu
        test_menu_item.set_enabled(false);
        test_menu_item.set_title("Menu disabled");
        test_menu_item.set_selected(true);
        #[cfg(target_os = "macos")]
        test_menu_item.set_native_image(NativeImage::StatusUnavailable);
      }
      _ => (),
    }
  });
}
