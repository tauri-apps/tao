// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// System tray is supported and availabled only if `tray` feature is enabled.
// Platform: Windows, Linux and macOS.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(feature = "tray")]
fn main() -> Result<(), tao::error::OsError> {
  use std::collections::HashMap;
  #[cfg(target_os = "linux")]
  use std::path::Path;
  use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    menu::{CustomMenuItem, Menu},
    system_tray::SystemTrayBuilder,
    window::{Window, WindowId},
  };

  env_logger::init();
  let event_loop = EventLoop::new();
  let mut windows: HashMap<WindowId, Window> = HashMap::new();

  let tray_menu = Menu::new()?;

  let open_new_window_item = CustomMenuItem::new("Open new window", true, false, None)?;
  let focus_window_item = CustomMenuItem::new("Focus window", false, false, None)?;
  let quit_item = CustomMenuItem::new("Quit", true, false, None)?;
  tray_menu.add_custom_item(&open_new_window_item);
  tray_menu.add_custom_item(&focus_window_item);
  tray_menu.add_custom_item(&quit_item);

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

  // Menu is shown with left click on macOS and right click on Windows.
  #[cfg(target_os = "macos")]
  let mut system_tray = SystemTrayBuilder::new(icon.clone(), Some(tray_menu))
    .with_icon_as_template(true)
    .build(&event_loop)
    .unwrap();

  #[cfg(not(target_os = "macos"))]
  let mut system_tray = SystemTrayBuilder::new(icon.clone(), Some(tray_menu))
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, event_loop, control_flow| {
    *control_flow = ControlFlow::Wait;

    let mut create_window_or_focus = || {
      // if we already have one window, let's focus instead
      if !windows.is_empty() {
        for window in windows.values() {
          window.set_focus();
        }
        return;
      }

      let window = Window::new(event_loop).unwrap();
      windows.insert(window.id(), window);
      open_new_window_item.set_enabled(false);
      open_new_window_item.set_title("Window already open");
      open_new_window_item.set_selected(true);
      focus_window_item.set_enabled(true);
      system_tray.set_icon(new_icon.clone());
    };

    match event {
      Event::WindowEvent {
        event, window_id, ..
      } => {
        if event == WindowEvent::CloseRequested {
          windows.remove(&window_id);
          open_new_window_item.set_enabled(true);
          focus_window_item.set_enabled(false);
          open_new_window_item.set_title("Open new window");
          open_new_window_item.set_selected(false);
          system_tray.set_icon(icon.clone());
        }
      }
      // on Windows, habitually, we show the window with left click
      #[cfg(target_os = "windows")]
      Event::TrayEvent {
        event: tao::event::TrayEvent::LeftClick,
        ..
      } => create_window_or_focus(),
      Event::MenuEvent { menu_id, .. } => {
        if menu_id == open_new_window_item.id() || menu_id == focus_window_item.clone().id() {
          create_window_or_focus();
        }

        if menu_id == quit_item.id() {
          *control_flow = ControlFlow::Exit;
        }
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
