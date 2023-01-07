// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// System tray is supported and availabled only if `tray` feature is enabled.
// Platform: Windows, Linux and macOS.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(any(feature = "tray", all(target_os = "linux", feature = "ayatana")))]
fn main() {
  #[cfg(target_os = "linux")]
  use tao::platform::linux::SystemTrayBuilderExtLinux;
  use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoop},
    menu::{ContextMenu as Menu, MenuItemAttributes, MenuType},
    system_tray::SystemTrayBuilder,
    TrayId,
  };

  #[cfg(target_os = "macos")]
  use tao::platform::macos::{SystemTrayBuilderExtMacOS, SystemTrayExtMacOS};

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
  let menu_item = tray_menu.add_item(MenuItemAttributes::new("Set tray title (macos)"));

  #[cfg(target_os = "macos")]
  {
    tray_menu
      .add_item(MenuItemAttributes::new("Menu Item with icon"))
      .set_icon(icon.clone());
  }

  let quit = tray_menu.add_item(MenuItemAttributes::new("Quit"));

  #[cfg(target_os = "linux")]
  let system_tray = SystemTrayBuilder::new(icon.clone(), Some(tray_menu))
    .with_id(main_tray_id)
    .with_temp_icon_dir(std::path::Path::new("/tmp/tao-examples"))
    .build(&event_loop)
    .unwrap();

  #[cfg(target_os = "windows")]
  let system_tray = SystemTrayBuilder::new(icon.clone(), Some(tray_menu))
    .with_id(main_tray_id)
    .with_tooltip("tao - windowing creation library")
    .build(&event_loop)
    .unwrap();

  #[cfg(target_os = "macos")]
  let system_tray = SystemTrayBuilder::new(icon.clone(), Some(tray_menu))
    .with_id(main_tray_id)
    .with_tooltip("tao - windowing creation library")
    .with_title("Tao")
    .build(&event_loop)
    .unwrap();

  let mut second_tray_menu = Menu::new();
  let log = second_tray_menu.add_item(MenuItemAttributes::new("Log"));
  let mut second_tray_menu = Some(second_tray_menu);

  let mut system_tray = Some(system_tray);
  let mut second_system_tray = None;

  event_loop.run(move |event, event_loop, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::NewEvents(StartCause::Init) => {
        let tray = SystemTrayBuilder::new(icon.clone(), second_tray_menu.take())
          .with_id(second_tray_id)
          .build(&event_loop)
          .unwrap();
        second_system_tray.replace(tray);
      }
      Event::MenuEvent {
        menu_id,
        // specify only context menu's
        origin: MenuType::ContextMenu,
        ..
      } => {
        if menu_id == quit.clone().id() {
          // drop the system tray before exiting to remove the icon from system tray on Windows
          system_tray.take();
          second_system_tray.take();
          *control_flow = ControlFlow::Exit;
        } else if menu_id == log.clone().id() {
          println!("Log clicked");
        } else if menu_id == menu_item.clone().id() {
          #[cfg(target_os = "macos")]
          {
            if let Some(tray) = system_tray.as_mut() {
              tray.set_title("Tao - clicked");
            }
          }
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
