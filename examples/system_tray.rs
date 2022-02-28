// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// System tray is supported and availabled only if `tray` feature is enabled.
// Platform: Windows, Linux and macOS.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(any(feature = "tray", all(target_os = "linux", feature = "ayatana")))]
fn main() {
  use std::collections::HashMap;
  #[cfg(target_os = "macos")]
  use tao::platform::macos::{CustomMenuItemExtMacOS, NativeImage, SystemTrayBuilderExtMacOS};
  use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    menu::{ContextMenu as Menu, MenuItemAttributes, MenuType},
    system_tray::SystemTrayBuilder,
    window::{Window, WindowId},
  };

  env_logger::init();
  let event_loop = EventLoop::new();
  let mut windows: HashMap<WindowId, Window> = HashMap::new();

  let mut tray_menu = Menu::new();
  let mut submenu = Menu::new();

  // open new window menu item
  let open_new_window_element = submenu.add_item(MenuItemAttributes::new("Open new window"));

  // set default icon
  #[cfg(target_os = "macos")]
  open_new_window_element
    .clone()
    .set_native_image(NativeImage::StatusAvailable);

  // focus all window menu item
  let mut focus_all_window =
    tray_menu.add_item(MenuItemAttributes::new("Focus window").with_enabled(false));

  let change_menu = tray_menu.add_item(MenuItemAttributes::new("Change menu"));

  // inject submenu into tray_menu
  tray_menu.add_submenu("Sub menu", true, submenu);

  // add quit button
  let quit_element = tray_menu.add_item(MenuItemAttributes::new("Quit"));

  let icon_slice = include_bytes!("icon.png");
  let icon = load_icon_from_slice(icon_slice);
  let new_icon_slice = include_bytes!("icon_blue.png");
  let new_icon = load_icon_from_slice(new_icon_slice);

  // Menu is shown with left click on macOS and right click on Windows.
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

      // create new window
      let mut open_new_window_element = open_new_window_element.clone();
      let mut focus_all_window = focus_all_window.clone();

      let window = Window::new(event_loop).unwrap();
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
      // add macOS Native red dot
      #[cfg(target_os = "macos")]
      open_new_window_element.set_native_image(NativeImage::StatusUnavailable);
    };

    match event {
      Event::WindowEvent {
        event, window_id, ..
      } => {
        if event == WindowEvent::CloseRequested {
          let mut open_new_window_element = open_new_window_element.clone();
          // Remove window from our hashmap
          windows.remove(&window_id);
          // Modify our button's state
          open_new_window_element.set_enabled(true);
          focus_all_window.set_enabled(false);
          // Reset text
          open_new_window_element.set_title("Open new window");
          // Set selected
          open_new_window_element.set_selected(false);
          // Change tray icon
          system_tray.set_icon(icon.clone());
          // macOS have native image available that we can use in our menu-items
          #[cfg(target_os = "macos")]
          open_new_window_element.set_native_image(NativeImage::StatusAvailable);
        }
      }
      // on Windows, habitually, we show the window with left click
      #[cfg(target_os = "windows")]
      Event::TrayEvent {
        event: tao::event::TrayEvent::LeftClick,
        ..
      } => create_window_or_focus(),
      // left click on menu item
      Event::MenuEvent {
        menu_id,
        // specify only context menu's
        origin: MenuType::ContextMenu,
        ..
      } => {
        // Click on Open new window or focus item
        if menu_id == open_new_window_element.clone().id()
          || menu_id == focus_all_window.clone().id()
        {
          create_window_or_focus();
        }
        // click on `quit` item
        if menu_id == quit_element.clone().id() {
          // tell our app to close at the end of the loop.
          *control_flow = ControlFlow::Exit;
        }

        if menu_id == change_menu.clone().id() {
          let mut tray_menu = Menu::new();
          tray_menu.add_item(MenuItemAttributes::new("Quit"));
          system_tray.set_menu(&tray_menu);
        }

        println!("Clicked on {:?}", menu_id);
      }
      _ => (),
    }
  });
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(any(feature = "tray", all(target_os = "linux", feature = "ayatana")))]
fn load_icon_from_slice(slice: &[u8]) -> tao::system_tray::Icon {
  let (icon_rgba, icon_width, icon_height) = {
    let image = image::load_from_memory(slice)
      .expect("Failed to parse icon slice")
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
