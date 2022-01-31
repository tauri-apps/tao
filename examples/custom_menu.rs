// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use tao::{
  accelerator::{Accelerator, SysMods},
  error::OsError,
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  keyboard::KeyCode,
  menu::{CustomMenuItem, Menu, NativeMenuItem},
  window::WindowBuilder,
};

fn main() -> Result<(), OsError> {
  env_logger::init();
  let event_loop = EventLoop::new();

  let menu_bar = Menu::new()?;

  let file_menu = Menu::with_title("File")?;
  let edit_menu = Menu::with_title("Edit")?;

  menu_bar.add_submenu(&file_menu);
  menu_bar.add_submenu(&edit_menu);

  let open_item = CustomMenuItem::new(
    "Open File...",
    true,
    false,
    Some(Accelerator::new(SysMods::Cmd, KeyCode::KeyO)),
  )?;
  let save_item = CustomMenuItem::new(
    "Save",
    false,
    false,
    Some(Accelerator::new(SysMods::Cmd, KeyCode::KeyS)),
  )?;
  let mut save_item_is_enabled = false;
  let toggle_save_item = CustomMenuItem::new(
    "Toggle Save menu item",
    true,
    false,
    Some(Accelerator::new(SysMods::Cmd, KeyCode::KeyD)),
  )?;
  file_menu.add_custom_item(&open_item);
  file_menu.add_custom_item(&save_item);
  file_menu.add_native_item(NativeMenuItem::Separator);
  file_menu.add_custom_item(&toggle_save_item);

  let custom_copy = CustomMenuItem::new(
    "Custom Copy",
    true,
    false,
    Some(Accelerator::new(SysMods::Cmd, KeyCode::KeyP)),
  )?;
  let add_new_items = CustomMenuItem::new("Add new menu item", true, false, None)?;
  edit_menu.add_native_item(NativeMenuItem::Copy);
  edit_menu.add_custom_item(&custom_copy);
  edit_menu.add_custom_item(&add_new_items);
  edit_menu.add_custom_item(&save_item);

  let mut window = WindowBuilder::new()
    .with_title("A fantastic window!")
    .with_menu(menu_bar)
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
      Event::MenuEvent { menu_id, .. } => match menu_id {
        _ if menu_id == open_item.id() => println!("Opened a file!"),

        _ if menu_id == save_item.id() => println!("Saved a file!"),

        _ if menu_id == toggle_save_item.id() => {
          save_item_is_enabled = !save_item_is_enabled;
          save_item.set_enabled(save_item_is_enabled);
          save_item.set_title(if save_item_is_enabled {
            "Save"
          } else {
            "Save (disabled)"
          });
          save_item.set_selected(!save_item_is_enabled);
        }

        _ if menu_id == custom_copy.id() => {
          println!("Copied(custom) some text!");
          window.set_menu(Some(file_menu.clone()))
        }

        _ if menu_id == add_new_items.id() => {
          let submenu = Menu::with_title("Submenu").unwrap();
          let item = CustomMenuItem::new(
            "New Menu Item",
            true,
            false,
            Some(Accelerator::new(SysMods::CmdShift, KeyCode::KeyM)),
          )
          .unwrap();
          let item2 = CustomMenuItem::new("New Menu Item2", true, false, None).unwrap();
          submenu.add_custom_item(&item);
          submenu.add_native_item(NativeMenuItem::Separator);
          submenu.add_custom_item(&item2);
          edit_menu.add_submenu(&submenu);
        }

        _ => println!("{menu_id}"),
      },
      _ => (),
    }
  });
}
