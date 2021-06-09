// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use simple_logger::SimpleLogger;
use tao::{
  accelerator::{Accelerator, RawMods},
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  keyboard::Key,
  platform::global_shortcut::ShortcutManager,
  window::WindowBuilder,
};

fn main() {
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();

  // create new shortcut manager instance
  let mut hotkey_manager = ShortcutManager::new();

  // create our accelerators
  let shortcut_f13 = Accelerator::new(None, Key::F13);
  let shortcut_altctrlmeta_b = Accelerator::new(RawMods::AltCtrlMeta, "b");

  // save a reference to unregister it later
  let global_shortcut_f13 = hotkey_manager.register(shortcut_f13.clone()).unwrap();

  hotkey_manager
    .register(shortcut_altctrlmeta_b.clone())
    .unwrap();

  // connect the event loop to the shortcut manager
  // (required only on linux) other platforms is no-op, it's safe to use it
  hotkey_manager.connect_event_loop(&event_loop).unwrap();

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
        global_shortcut_f13.clone().unregister();
      }
      Event::GlobalShortcutEvent(hotkey_id) if hotkey_id == shortcut_altctrlmeta_b.clone().id() => {
        println!("Pressed on Alt + Ctrl + Meta + b");
      }
      _ => (),
    }
  });
}
