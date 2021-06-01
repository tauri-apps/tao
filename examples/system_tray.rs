// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(
  target_os = "macos",
  target_os = "windows",
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]
fn main() {
  use simple_logger::SimpleLogger;
  use std::collections::HashMap;
  #[cfg(target_os = "linux")]
  use std::path::Path;
  #[cfg(target_os = "macos")]
  use tao::platform::macos::NativeImage;
  use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    menu::{MenuItem, MenuType, Tray as Menu},
    platform::system_tray::SystemTrayBuilder,
    window::Window,
  };

  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();
  let mut windows = HashMap::new();

  let mut tray_menu = Menu::new();

  let mut submenu = Menu::new();

  // open new window menu item
  let open_new_window_element = submenu.add_item(MenuItem::new("Open new window"));

  // set default icon
  #[cfg(target_os = "macos")]
  open_new_window_element
    .clone()
    .unwrap()
    .set_icon(NativeImage::StatusAvailable);

  // focus all window menu item
  let focus_all_window = tray_menu.add_item(MenuItem::new("Focus window")).unwrap();

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
          println!("Window {:?} has received the signal to close", window_id);
          let mut open_new_window_element = open_new_window_element.clone().unwrap();
          // Remove window from our hashmap
          windows.remove(&window_id);
          // Enable our button
          open_new_window_element.set_enabled(true);
          // Reset text
          open_new_window_element.set_title("Open new window");
          // Set unchecked
          open_new_window_element.set_selected(false);
          system_tray.set_icon(icon.clone());
          #[cfg(target_os = "macos")]
          open_new_window_element.set_icon(NativeImage::StatusAvailable);
        }
      }
      Event::MenuEvent {
        menu_id,
        origin: MenuType::SystemTray,
      } => {
        let mut open_new_window_element = open_new_window_element.clone().unwrap();
        if menu_id == open_new_window_element.clone().id() {
          let window = Window::new(&event_loop).unwrap();
          windows.insert(window.id(), window);
          // disable button
          open_new_window_element.set_enabled(false);
          // change title (text)
          open_new_window_element.set_title("Window already open");
          // set checked
          open_new_window_element.set_selected(true);

          // update tray icon
          system_tray.set_icon(new_icon.clone());
          #[cfg(target_os = "macos")]
          open_new_window_element.set_icon(NativeImage::StatusUnavailable);
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

#[cfg(any(target_os = "ios", target_os = "android",))]
fn main() {
  println!("This platform doesn't support run_return.");
}
