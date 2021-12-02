// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
fn main() {
  use std::str::FromStr;
  use tao::{
    accelerator::{Accelerator, AcceleratorId, RawMods, SysMods},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    global_shortcut::ShortcutManager,
    keyboard::KeyCode,
    window::WindowBuilder,
  };

  env_logger::init();
  let event_loop = EventLoop::new();

  // create new shortcut manager instance
  let mut hotkey_manager = ShortcutManager::new(&event_loop);

  // create our accelerators
  let shortcut_1 = Accelerator::new(SysMods::Shift, KeyCode::ArrowUp);
  let shortcut_2 = Accelerator::new(RawMods::AltCtrlMeta, KeyCode::KeyB);
  // use string parser to generate accelerator (require `std::str::FromStr`)
  let shortcut_3 = Accelerator::from_str("COMMANDORCONTROL+SHIFT+3").unwrap();
  let shortcut_4 = Accelerator::from_str("COMMANDORCONTROL+shIfT+DOWN").unwrap();

  // save a reference to unregister it later
  let global_shortcut_1 = hotkey_manager.register(shortcut_1.clone()).unwrap();
  // register other accelerator's
  hotkey_manager.register(shortcut_2.clone()).unwrap();
  hotkey_manager.register(shortcut_3).unwrap();
  hotkey_manager.register(shortcut_4.clone()).unwrap();

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
        ..
      } if window_id == window.id() => *control_flow = ControlFlow::Exit,
      Event::MainEventsCleared => {
        window.request_redraw();
      }
      Event::GlobalShortcutEvent(hotkey_id) if hotkey_id == shortcut_1.clone().id() => {
        println!("Pressed `shortcut_1` -- unregister for future use");
        // unregister key
        hotkey_manager
          .unregister(global_shortcut_1.clone())
          .unwrap();
      }
      Event::GlobalShortcutEvent(hotkey_id) if hotkey_id == shortcut_2.clone().id() => {
        println!("Pressed on `shortcut_2`");
      }
      // you can match hotkey_id with accelerator_string only if you used `from_str`
      // by example `shortcut_1` will NOT match AcceleratorId::new("SHIFT+UP") as it's
      // been created with a struct and the ID is generated automatically
      Event::GlobalShortcutEvent(hotkey_id)
        if hotkey_id == AcceleratorId::new("COMMANDORCONTROL+SHIFT+3") =>
      {
        println!("Pressed on `shortcut_3`");
      }
      Event::GlobalShortcutEvent(hotkey_id) if hotkey_id == shortcut_4.clone().id() => {
        println!("Pressed on `shortcut_4`");
      }
      Event::GlobalShortcutEvent(hotkey_id) => {
        println!("hotkey_id {:?}", hotkey_id);
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
