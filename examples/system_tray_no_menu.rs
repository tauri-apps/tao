// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// System tray is supported and availabled only if feature flag is enabled.
// Platform: Windows, Linux and macOS.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(feature = "tray")]
fn main() {
  use std::collections::HashMap;
  #[cfg(target_os = "linux")]
  use std::path::Path;
  #[cfg(target_os = "linux")]
  use tao::menu::{ContextMenu, MenuItemAttributes};
  #[cfg(target_os = "macos")]
  use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
  #[cfg(target_os = "linux")]
  use tao::platform::unix::WindowBuilderExtUnix;
  #[cfg(target_os = "windows")]
  use tao::platform::windows::WindowBuilderExtWindows;
  use tao::{
    dpi::LogicalSize,
    event::{Event, Rectangle, TrayEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    menu::MenuId,
    system_tray::SystemTrayBuilder,
    window::WindowBuilder,
  };

  env_logger::init();
  #[cfg(target_os = "macos")]
  let mut event_loop = EventLoop::new();

  #[cfg(not(target_os = "macos"))]
  let event_loop = EventLoop::new();

  // launch macos app without menu and without dock icon
  // shouold be set at launch
  #[cfg(target_os = "macos")]
  event_loop.set_activation_policy(ActivationPolicy::Accessory);

  let mut windows = HashMap::new();

  fn window_position_center_tray(
    rectangle: &mut Rectangle,
    window_size: LogicalSize<f64>,
  ) -> Rectangle {
    // center X axis with tray icon position
    let window_x =
      rectangle.position.x + ((rectangle.size.width / 2.0) - (window_size.width / 2.0));
    rectangle.position.x = window_x;

    // position Y axis (Windows only)
    #[cfg(target_os = "windows")]
    {
      rectangle.position.y = rectangle.position.y - window_size.height - rectangle.size.height;
    }

    *rectangle
  }

  // Windows require Vec<u8> ICO file
  #[cfg(target_os = "windows")]
  let icon = include_bytes!("icon.ico").to_vec();
  // macOS require Vec<u8> PNG file
  #[cfg(target_os = "macos")]
  let icon = include_bytes!("icon.png").to_vec();
  // Linux require Pathbuf to PNG file
  #[cfg(target_os = "linux")]
  let icon = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/icon.png");

  // linux require a menu so let's add only a open button
  #[cfg(target_os = "linux")]
  let open_menu_id = MenuId::new("open_menu");
  let quit_menu_id = MenuId::new("quit_menu");
  #[cfg(target_os = "linux")]
  {
    let mut menu = ContextMenu::new();
    menu.add_item(MenuItemAttributes::new("Open").with_id(open_menu_id));
    menu.add_item(MenuItemAttributes::new("Quit").with_id(quit_menu_id));
    let _system_tray = SystemTrayBuilder::new(icon, Some(menu))
      .build(&event_loop)
      .unwrap();
  }

  // macos and windows support no menu in tray
  #[cfg(any(target_os = "macos", target_os = "windows"))]
  let _system_tray = SystemTrayBuilder::new(icon, None)
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, event_loop, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent {
        event, window_id, ..
      } => {
        if event == WindowEvent::CloseRequested {
          println!("Window {:?} has received the signal to close", window_id);
          // Remove window from our hashmap
          windows.remove(&window_id);
        }
      }
      // if we got a menu event from our `open_menu_id` open a new window..
      // (used on linux)
      #[cfg(target_os = "linux")]
      Event::MenuEvent { menu_id, .. } if menu_id == open_menu_id => {
        let window = WindowBuilder::new()
          .with_skip_taskbar(true)
          .build(event_loop)
          .unwrap();
        windows.insert(window.id(), window);
      }
      // if we got `Quit` click, exit the app
      Event::MenuEvent { menu_id, .. } if menu_id == quit_menu_id => {
        *control_flow = ControlFlow::Exit
      }
      Event::TrayEvent {
        mut bounds,
        event,
        position: _cursor_position,
        ..
      } => {
        if event == TrayEvent::LeftClick {
          println!("{:?}", bounds.position);
          if windows.is_empty() {
            // window size
            let window_inner_size = LogicalSize::new(200.0, 200.0);
            // create our window
            let mut window_builder = WindowBuilder::new();
            window_builder = window_builder
              // position our window centered with tray bounds
              .with_position(window_position_center_tray(&mut bounds, window_inner_size).position)
              .with_inner_size(window_inner_size)
              // disallow resize
              .with_resizable(false);

            // skip taskbar on windows & linux
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            {
              window_builder = window_builder.with_skip_taskbar(true);
            }

            let window = window_builder.build(event_loop).unwrap();
            windows.insert(window.id(), window);
          } else {
            for window in windows.values() {
              window.set_focus();
            }
          }
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
