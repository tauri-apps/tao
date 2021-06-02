// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// System tray is supported and availabled only if feature flag is enabled.
// Platform: Windows, Linux and macOS.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(feature = "tray")]
fn main() {
  use simple_logger::SimpleLogger;
  use std::collections::HashMap;
  #[cfg(target_os = "linux")]
  use std::path::Path;
  #[cfg(target_os = "macos")]
  use tao::platform::macos::{CustomMenuItemExtMacOS, NativeImage};
  use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    menu::{ContextMenu as Menu, MenuType},
    system_tray::SystemTrayBuilder,
    window::Window,
  };

  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();
  let mut windows = HashMap::new();

  let mut tray_menu = Menu::new();

  let mut submenu = Menu::new();

  // open new window menu item
  let open_new_window_element = submenu.add_item("Open new window", None, true, false);

  // set default icon
  #[cfg(target_os = "macos")]
  open_new_window_element
    .clone()
    .set_native_image(NativeImage::StatusAvailable);

  // focus all window menu item
  let mut focus_all_window = tray_menu.add_item("Focus window", None, false, false);

  // inject submenu into tray_menu
  tray_menu.add_submenu("Sub menu", true, submenu);

  // Windows require Vec<u8> ICO file
  #[cfg(target_os = "windows")]
  let icon = include_bytes!("icon.ico").to_vec();
  // macOS require Vec<u8> PNG file
  #[cfg(target_os = "macos")]
  let icon = include_bytes!("icon.png").to_vec();
  // Linux require Pathbuf to PNG file
  #[cfg(target_os = "linux")]
  let icon = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/icon.png");

  // Windows require Vec<u8> ICO file
  #[cfg(target_os = "windows")]
  let new_icon = include_bytes!("icon_blue.ico").to_vec();
  // macOS require Vec<u8> PNG file
  #[cfg(target_os = "macos")]
  let new_icon = include_bytes!("icon_dark.png").to_vec();
  // Linux require Pathbuf to PNG file
  #[cfg(target_os = "linux")]
  let new_icon = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/icon_dark.png");

  // Only supported on macOS, linux and windows
  #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
  let mut system_tray = SystemTrayBuilder::new(icon.clone(), Some(tray_menu))
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, event_loop, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent { event, window_id } => {
        if event == WindowEvent::CloseRequested {
          let mut open_new_window_element = open_new_window_element.clone();
          println!("Window {:?} has received the signal to close", window_id);
          // Remove window from our hashmap
          windows.remove(&window_id);
          // Enable our button
          open_new_window_element.set_enabled(true);
          focus_all_window.set_enabled(false);
          // Reset text
          open_new_window_element.set_title("Open new window");
          // Set unchecked
          open_new_window_element.set_selected(false);
          system_tray.set_icon(icon.clone());
          #[cfg(target_os = "macos")]
          open_new_window_element.set_native_image(NativeImage::StatusAvailable);
        }
      }
      Event::MenuEvent {
        menu_id,
        origin: MenuType::ContextMenu,
      } => {
        let mut open_new_window_element = open_new_window_element.clone();
        if menu_id == open_new_window_element.clone().id() {
          let window = Window::new(&event_loop).unwrap();
          windows.insert(window.id(), window);
          // disable button
          open_new_window_element.set_enabled(false);
          // change title (text)
          open_new_window_element.set_title("Window already open");
          // set checked
          open_new_window_element.set_selected(true);
          // enable focus window
          focus_all_window.set_enabled(true);
          // update tray icon
          system_tray.set_icon(new_icon.clone());
          #[cfg(target_os = "macos")]
          open_new_window_element.set_native_image(NativeImage::StatusUnavailable);
        }
        if menu_id == focus_all_window.clone().id() {
          for window in windows.values() {
            window.set_focus();
          }
        }
        println!("Clicked on {:?}", menu_id);
      }
      _ => (),
    }
  });
}

// System tray isn't supported on other's platforms.
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn main() {
  println!("This platform doesn't support system_tray.");
}

// Tray feature flag disabled but can be available.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(not(feature = "tray"))]
fn main() {
  println!("This platform doesn't have the `tray` feature enabled.");
}
