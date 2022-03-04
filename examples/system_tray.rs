// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// System tray is supported and availabled only if `tray` feature is enabled.
// Platform: Windows, Linux and macOS.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(feature = "tray")]
fn main() {
  use std::collections::HashMap;
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

  let tray_menu = Menu::new();

  let open_new_window_item = CustomMenuItem::new("Open new window", true, false, None);
  let focus_window_item = CustomMenuItem::new("Focus window", false, false, None);
  let quit_item = CustomMenuItem::new("Quit", true, false, None);
  tray_menu.add_custom_item(&open_new_window_item);
  tray_menu.add_custom_item(&focus_window_item);
  tray_menu.add_custom_item(&quit_item);

  let icon_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/icon.png");
  let new_icon_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/new_icon.png");

  let icon = load_icon(Path::new(icon_path));
  let new_icon = load_icon(Path::new(new_icon_path));

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

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(feature = "tray")]
fn load_icon(path: &std::path::Path) -> tao::system_tray::Icon {
  let (icon_rgba, icon_width, icon_height) = {
    let image = image::open(path)
      .expect("Failed to open icon path")
      .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    (rgba, width, height)
  };
  tao::system_tray::Icon::from_rgba(icon_rgba, icon_width, icon_height)
    .expect("Failed to open icon")
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
