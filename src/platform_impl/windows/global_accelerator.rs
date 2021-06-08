use crate::{
  event_loop::EventLoopWindowTarget,
  hotkey::{GlobalAccelerator as RootGlobalAccelerator, HotKey},
  keyboard::ModifiersState,
  platform::scancode::KeyCodeExtScancode,
};

use super::keyboard::key_to_vk;
use winapi::{shared::windef::HWND, um::winuser};

pub struct GlobalAccelerator {
  pub(crate) hotkey: HotKey,
}

impl GlobalAccelerator {
  pub(crate) fn new(hotkey: HotKey) -> Self {
    Self { hotkey }
  }

  pub(crate) fn register(&mut self) -> &mut GlobalAccelerator {
    let mut converted_modifiers: u32 = 0;
    let modifiers: ModifiersState = self.hotkey.mods.into();
    if modifiers.shift_key() {
      converted_modifiers |= winuser::MOD_SHIFT as u32;
    }
    if modifiers.super_key() {
      converted_modifiers |= winuser::MOD_WIN as u32;
    }
    if modifiers.alt_key() {
      converted_modifiers |= winuser::MOD_ALT as u32;
    }
    if modifiers.control_key() {
      converted_modifiers |= winuser::MOD_CONTROL as u32;
    }

    // get key scan code
    if let Some(vk_code) = key_to_vk(&self.hotkey.key) {
      println!("vk_code {:?}", vk_code);
    };

    self
  }
}

pub fn register_global_accelerators<T>(
  _window_target: &EventLoopWindowTarget<T>,
  accelerators: &mut Vec<RootGlobalAccelerator>,
) {
  for accel in accelerators {
    accel.0.register();
  }
}
