use super::keyboard::key_to_vk;
use crate::{
  accelerator::Accelerator,
  event_loop::EventLoopWindowTarget,
  keyboard::ModifiersState,
  platform::{
    global_shortcut::{GlobalShortcut as RootGlobalShortcut, ShortcutManagerError},
    scancode::KeyCodeExtScancode,
  },
};
use std::ptr;
use winapi::{shared::windef::HWND, um::winuser};

#[derive(Debug, Clone)]
pub struct ShortcutManager {
  shortcuts: Vec<GlobalShortcut>,
}

impl ShortcutManager {
  pub(crate) fn new<T>(_window_target: &EventLoopWindowTarget<T>) -> Self {
    ShortcutManager {
      shortcuts: Vec::new(),
    }
  }

  pub(crate) fn register(
    &mut self,
    accelerator: Accelerator,
  ) -> Result<RootGlobalShortcut, ShortcutManagerError> {
    unsafe {
      let mut converted_modifiers: u32 = 0;
      let modifiers: ModifiersState = accelerator.mods.into();
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
      match key_to_vk(&accelerator.key) {
        Some(vk_code) => unsafe {
          let result = winuser::RegisterHotKey(
            ptr::null_mut(),
            accelerator.clone().id() as i32,
            converted_modifiers,
            vk_code as u32,
          );
          if result == 0 {
            return Err(ShortcutManagerError::InvalidAccelerator(
              "Unable to register accelerator with `winuser::RegisterHotKey`.".into(),
            ));
          }
          let shortcut = GlobalShortcut { accelerator };
          self.shortcuts.push(shortcut.clone());
          return Ok(RootGlobalShortcut(shortcut));
        },
        _ => {
          return Err(ShortcutManagerError::InvalidAccelerator(
            "Unable to register accelerator (unknown VKCode for this char).".into(),
          ));
        }
      }
    }
  }

  pub(crate) fn unregister_all(&self) -> Result<(), ShortcutManagerError> {
    for shortcut in &self.shortcuts {
      shortcut.unregister();
    }
    Ok(())
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalShortcut {
  pub(crate) accelerator: Accelerator,
}

impl GlobalShortcut {
  pub fn id(&self) -> u16 {
    self.accelerator.clone().id()
  }
}
