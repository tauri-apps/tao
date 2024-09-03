// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_os = "macos")]
use tao::platform::macos::{CustomMenuItemExtMacOS, NativeImage};
use tao::{
  accelerator::{Accelerator, SysMods},
  clipboard::Clipboard,
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  keyboard::KeyCode,
  menu::{AboutMetadata, MenuBar as Menu, MenuItem, MenuItemAttributes, MenuType, SharingItem},
  window::WindowBuilder,
};

fn main() {
  env_logger::init();
  let event_loop = EventLoop::new();

  // create clipboard instance
  let mut cliboard = Clipboard::new();

  // create main menubar menu
  let mut menu_bar_menu = Menu::new();

  // create `first_menu`
  let mut first_menu = Menu::new();

  // create second menu
  let mut second_menu = Menu::new();
  // create an empty menu to be used as submenu
  let mut my_sub_menu = Menu::new();

  let mut test_menu_item = my_sub_menu.add_item(
    MenuItemAttributes::new("Disable menu")
      .with_accelerators(&Accelerator::new(SysMods::Cmd, KeyCode::KeyD)),
  );

  // add native `Copy` to `first_menu` menu
  // in macOS native item are required to get keyboard shortcut
  // to works correctly
  first_menu.add_native_item(MenuItem::Copy);
  first_menu.add_native_item(MenuItem::About(
    "tao".into(),
    AboutMetadata {
      version: Some("1.0.0".into()),
      ..Default::default()
    },
  ));

  first_menu.add_native_item(MenuItem::Services);
  first_menu.add_native_item(MenuItem::Share(SharingItem {
    texts: Some(vec!["Foo".to_string(), " Bar!".to_string()]),
    urls: Some(vec!["https://tauri.app".to_string()]),
    file_paths: Some(vec![std::path::PathBuf::from(concat!(
      env!("CARGO_MANIFEST_DIR"),
      "/examples/icon.png"
    ))]),
  }));

  // Create custom Copy menu with our clipboard object
  let custom_insert_clipboard = first_menu.add_item(MenuItemAttributes::new("&Insert clipboard"));
  let custom_read_clipboard = first_menu.add_item(MenuItemAttributes::new("&Read clipboard"));

  // add `my_sub_menu` children of `first_menu` with `Sub menu` title
  first_menu.add_submenu("&Sub menu", true, my_sub_menu);
  first_menu.add_native_item(MenuItem::CloseWindow);
  first_menu.add_native_item(MenuItem::Hide);
  first_menu.add_native_item(MenuItem::Quit);

  // create custom item `Selected and disabled` children of `second_menu`
  second_menu.add_item(
    MenuItemAttributes::new("Selected &and disabled")
      .with_selected(true)
      .with_enabled(false),
  );
  // add separator in `second_menu`
  second_menu.add_native_item(MenuItem::Separator);
  // create custom item `Change menu` children of `second_menu`
  let change_menu = second_menu.add_item(MenuItemAttributes::new("Change menu"));

  second_menu.add_native_item(MenuItem::Separator);
  let zoom_in_item = second_menu.add_item(
    MenuItemAttributes::new("Zoom in").with_accelerators(&"CmdOrCtrl+Plus".parse().unwrap()),
  );

  // add all our childs to menu_bar_menu (order is how they'll appear)
  menu_bar_menu.add_submenu("&My app", true, first_menu);
  menu_bar_menu.add_submenu("&Other menu", true, second_menu);

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
        ..
      } if window_id == window.id() => *control_flow = ControlFlow::Exit,
      Event::MenuEvent {
        window_id,
        menu_id,
        origin: MenuType::MenuBar,
        ..
      } if menu_id == test_menu_item.clone().id() => {
        println!("Clicked on `Disable menu`");
        if window_id == Some(window.id()) {
          println!("Window ID match!");
        }
        // this allow us to get access to the menu and make changes
        // without re-rendering the whole menu
        test_menu_item.set_enabled(false);
        test_menu_item.set_title("Menu disabled");
        test_menu_item.set_selected(true);
        #[cfg(target_os = "macos")]
        test_menu_item.set_native_image(NativeImage::StatusUnavailable);
      }
      Event::MenuEvent {
        menu_id,
        origin: MenuType::MenuBar,
        ..
      } if menu_id == change_menu.clone().id() => {
        println!("Clicked on `Change menu`");
        // set new menu
        let mut menu_bar_menu = Menu::new();
        let mut my_app_menu = Menu::new();
        my_app_menu.add_item(MenuItemAttributes::new("New menu!"));
        menu_bar_menu.add_submenu("My app", true, my_app_menu);
        window.set_menu(Some(menu_bar_menu))
      }
      Event::MenuEvent {
        menu_id,
        origin: MenuType::MenuBar,
        ..
      } if menu_id == custom_insert_clipboard.clone().id() => {
        cliboard.write_text("This is injected from tao!!!")
      }
      Event::MenuEvent {
        menu_id,
        origin: MenuType::MenuBar,
        ..
      } if menu_id == custom_read_clipboard.clone().id() => {
        println!("Clipboard content: {:?}", cliboard.read_text());
      }
      Event::MenuEvent {
        menu_id,
        origin: MenuType::MenuBar,
        ..
      } if menu_id == zoom_in_item.clone().id() => {
        println!("Zoom in!");
      }
      _ => (),
    }
  });
}
