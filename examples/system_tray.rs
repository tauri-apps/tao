// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// System tray is supported and availabled only if `tray` feature is enabled.
// Platform: Windows, Linux and macOS.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(any(feature = "tray", all(target_os = "linux", feature = "ayatana")))]
fn main() {
  #[cfg(target_os = "linux")]
  use tao::platform::linux::SystemTrayBuilderExtLinux;
  use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    menu::{ContextMenu as Menu, MenuItemAttributes, MenuType},
    system_tray::SystemTrayBuilder,
    TrayId,
  };

  env_logger::init();
  let event_loop = EventLoop::new();

  // You'll have to choose an icon size at your own discretion. On Linux, the icon should be
  // provided in whatever size it was naturally drawn; that is, don’t scale the image before passing
  // it to Tao. But on Windows, you will have to account for screen scaling. Here we use 32px,
  // since it seems to work well enough in most cases. Be careful about going too high, or
  // you'll be bitten by the low-quality downscaling built into the WM.
  let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/icon.png");

  let main_tray_id = TrayId::new("main-tray");
  let second_tray_id = TrayId::new("2nd-tray");
  let icon = load_icon(std::path::Path::new(path));
  let mut tray_menu = Menu::new();

  #[cfg(target_os = "macos")]
  {
    tray_menu
      .add_item(MenuItemAttributes::new("Item 1"))
      .set_icon(icon.clone());
  }

  let quit = tray_menu.add_item(MenuItemAttributes::new("Quit"));

  #[cfg(target_os = "linux")]
  let system_tray = SystemTrayBuilder::new_with_id(main_tray_id, icon.clone(), Some(tray_menu))
    .with_temp_icon_dir(std::path::Path::new("/tmp/tao-examples"))
    .build(&event_loop)
    .unwrap();

  #[cfg(not(target_os = "linux"))]
  let system_tray = SystemTrayBuilder::new_with_id(main_tray_id, icon.clone(), Some(tray_menu))
    .with_tooltip("tao - windowing creation library")
    .build(&event_loop)
    .unwrap();

  let mut second_tray_menu = Menu::new();
  let log = second_tray_menu.add_item(MenuItemAttributes::new("Log"));
  let second_system_tray =
    SystemTrayBuilder::new_with_id(second_tray_id, icon, Some(second_tray_menu))
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
          drop(&second_system_tray);
          *control_flow = ControlFlow::Exit;
        } else if menu_id == log.clone().id() {
          println!("Log clicked");
        }
      }
      Event::TrayEvent {
        id,
        bounds,
        event,
        position,
        ..
      } => {
        let tray = if id == main_tray_id {
          "main"
        } else if id == second_tray_id {
          "second"
        } else {
          "unknown"
        };
        println!(
          "tray `{}` event: {:?} {:?} {:?}",
          tray, event, bounds, position
        );
      }
      _ => (),
    }
  });
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(any(feature = "tray", all(target_os = "linux", feature = "ayatana")))]
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
