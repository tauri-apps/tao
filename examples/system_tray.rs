// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// System tray is supported and availabled only if `tray` feature is enabled.
// Platform: Windows, Linux and macOS.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(any(feature = "tray", all(target_os = "linux", feature = "ayatana")))]
fn main() {
  #[cfg(target_os = "macos")]
  use tao::platform::macos::{CustomMenuItemExtMacOS, NativeImage, SystemTrayBuilderExtMacOS};
  use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    menu::{ContextMenu as Menu, MenuItemAttributes, MenuType},
    system_tray::SystemTrayBuilder,
  };

  env_logger::init();
  let event_loop = EventLoop::new();

  let mut tray_menu = Menu::new();
  let quit = tray_menu.add_item(MenuItemAttributes::new("Quit"));

  #[cfg(target_os = "windows")]
  let icon = include_bytes!("tray_16.ico");
  #[cfg(target_os = "linux")]
  let icon = include_bytes!("tray_16.png");
  #[cfg(target_os = "macos")]
  let icon = include_bytes!("tray_18.png");

  let system_tray = SystemTrayBuilder::new(icon.to_vec(), Some(tray_menu))
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, _event_loop, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::MenuEvent {
        menu_id,
        // specify only context menu's
        origin: MenuType::ContextMenu,
        ..
      } => {
        if menu_id == quit.clone().id() {
          // drop the system tray before exiting to remove the icon from system tray on Windows
          drop(&system_tray);
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
