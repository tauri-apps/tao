// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use simple_logger::SimpleLogger;
use tao::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  hotkey::{HotKey, HotKeyManager, RawMods},
  keyboard::Key,
  window::WindowBuilder,
};

fn main() {
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();

  // register global hotkey without modifier
  let shortcut_f13 = HotKey::new(None, Key::F13);
  let shortcut_altctrlmeta_b = HotKey::new(RawMods::AltCtrlMeta, "b");

  let mut hotkey_manager = HotKeyManager::new();
  let registered_f13 = hotkey_manager.register(shortcut_f13.clone()).unwrap();
  hotkey_manager
    .register(shortcut_altctrlmeta_b.clone())
    .unwrap();

  hotkey_manager.run(&event_loop).unwrap();

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
      Event::GlobalHotKeyEvent(hotkey_id) if hotkey_id == shortcut_f13.clone().id() => {
        println!(
          "Pressed on F13 !!!  registered_f13 {:?}",
          registered_f13.clone().test()
        );
      }
      Event::GlobalHotKeyEvent(hotkey_id) if hotkey_id == shortcut_altctrlmeta_b.clone().id() => {
        println!("Pressed on Alt + Ctrl + Meta + b !!!");
      }
      _ => (),
    }
  });
}
