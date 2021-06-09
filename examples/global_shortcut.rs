// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
fn main() {
  use simple_logger::SimpleLogger;
  use tao::{
    accelerator::{Accelerator, RawMods},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::Key,
    platform::global_shortcut::ShortcutManager,
    window::WindowBuilder,
  };

  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();

  // create new shortcut manager instance
  let mut hotkey_manager = ShortcutManager::new(&event_loop);

  // create our accelerators
  let shortcut_f13 = Accelerator::new(None, Key::F13);
  let shortcut_altctrlmeta_b = Accelerator::new(RawMods::AltCtrlMeta, "b");

  // save a reference to unregister it later
  let global_shortcut_f13 = hotkey_manager.register(shortcut_f13.clone()).unwrap();

  hotkey_manager
    .register(shortcut_altctrlmeta_b.clone())
    .unwrap();

  let window = WindowBuilder::new()
    .with_title("A fantastic window!")
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        window_id,
      } if window_id == window.id() => *control_flow = ControlFlow::Exit,
      Event::MainEventsCleared => {
        window.request_redraw();
      }
      Event::GlobalShortcutEvent(hotkey_id) if hotkey_id == shortcut_f13.clone().id() => {
        println!("Pressed on F13 -- unregister for future use");
        // unregister key
        hotkey_manager
          .unregister(global_shortcut_f13.clone())
          .unwrap();
      }
      Event::GlobalShortcutEvent(hotkey_id) if hotkey_id == shortcut_altctrlmeta_b.clone().id() => {
        println!("Pressed on Alt + Ctrl + Meta + b");
      }
      _ => (),
    }
  });
}

// Global shortcut isn't supported on other's platforms.
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn main() {
  println!("This platform doesn't support global_shortcut.");
}
