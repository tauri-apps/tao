use super::{
  keyboard::key_to_raw_key,
  window::{WindowId, WindowRequest},
};
use crate::{
  accelerator::{Accelerator, AcceleratorId},
  event_loop::EventLoopWindowTarget,
  global_shortcut::{GlobalShortcut as RootGlobalShortcut, ShortcutManagerError},
  keyboard::{KeyCode, ModifiersState},
};
use gtk::AccelGroup;
use gtk::{prelude::*, ApplicationWindow};

#[derive(Debug)]
pub struct ShortcutManager {
  group: AccelGroup,
  tx: glib::Sender<AcceleratorId>,
}

impl ShortcutManager {
  pub(crate) fn new<T>(window_target: &EventLoopWindowTarget<T>) -> Self {
    let group = AccelGroup::new();
    let menu = gtk::MenuBar::new();
    window_target.p.app.set_menubar(Some(&menu));
    for w in window_target.p.app.windows() {
      w.add_accel_group(&group);
    }
    Self {
      group,
      tx: window_target.p.shortcut_requests_tx.clone(),
    }
  }

  pub(crate) fn register(
    &mut self,
    accelerator: Accelerator,
  ) -> Result<RootGlobalShortcut, ShortcutManagerError> {
    if let Some((accel_key, accel_mods)) = to_gtk_key(&accelerator) {
      let hotkey_id = accelerator.clone().id();
      let tx = self.tx.clone();
      self.group.connect_accel_group(
        accel_key,
        accel_mods,
        gtk::AccelFlags::VISIBLE,
        move |_, _, _, _| {
          if let Err(e) = tx.send(hotkey_id) {
            log::warn!(
              "Failed to send global shortcut event to event loop channel: {}",
              e
            );
          }
          true
        },
      );

      //self.gourp.connect_accel_
      Ok(RootGlobalShortcut(GlobalShortcut { accelerator }))
    } else {
      Err(ShortcutManagerError::InvalidAccelerator(format!(
        "{:?}",
        accelerator
      )))
    }
  }

  pub(crate) fn unregister_all(&mut self) -> Result<(), ShortcutManagerError> {
    AccelGroupExt::disconnect(&self.group, None);
    Ok(())
  }

  pub(crate) fn unregister(
    &self,
    shortcut: RootGlobalShortcut,
  ) -> Result<(), ShortcutManagerError> {
    let accelerator = shortcut.0.accelerator;
    if let Some((accel_key, accel_mods)) = to_gtk_key(&accelerator) {
      if self.group.disconnect_key(accel_key, accel_mods) {
        Ok(())
      } else {
        Err(ShortcutManagerError::AcceleratorNotRegistered(accelerator))
      }
    } else {
      Err(ShortcutManagerError::InvalidAccelerator(format!(
        "{:?}",
        accelerator,
      )))
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalShortcut {
  pub(crate) accelerator: Accelerator,
}

impl GlobalShortcut {
  pub fn id(&self) -> AcceleratorId {
    self.accelerator.clone().id()
  }
}

fn to_gtk_key(key: &Accelerator) -> Option<(u32, gdk::ModifierType)> {
  let accel_key = match &key.key {
    KeyCode::KeyA => 'A' as u32,
    KeyCode::KeyB => 'B' as u32,
    KeyCode::KeyC => 'C' as u32,
    KeyCode::KeyD => 'D' as u32,
    KeyCode::KeyE => 'E' as u32,
    KeyCode::KeyF => 'F' as u32,
    KeyCode::KeyG => 'G' as u32,
    KeyCode::KeyH => 'H' as u32,
    KeyCode::KeyI => 'I' as u32,
    KeyCode::KeyJ => 'J' as u32,
    KeyCode::KeyK => 'K' as u32,
    KeyCode::KeyL => 'L' as u32,
    KeyCode::KeyM => 'M' as u32,
    KeyCode::KeyN => 'N' as u32,
    KeyCode::KeyO => 'O' as u32,
    KeyCode::KeyP => 'P' as u32,
    KeyCode::KeyQ => 'Q' as u32,
    KeyCode::KeyR => 'R' as u32,
    KeyCode::KeyS => 'S' as u32,
    KeyCode::KeyT => 'T' as u32,
    KeyCode::KeyU => 'U' as u32,
    KeyCode::KeyV => 'V' as u32,
    KeyCode::KeyW => 'W' as u32,
    KeyCode::KeyX => 'X' as u32,
    KeyCode::KeyY => 'Y' as u32,
    KeyCode::KeyZ => 'Z' as u32,
    KeyCode::Digit0 => '0' as u32,
    KeyCode::Digit1 => '1' as u32,
    KeyCode::Digit2 => '2' as u32,
    KeyCode::Digit3 => '3' as u32,
    KeyCode::Digit4 => '4' as u32,
    KeyCode::Digit5 => '5' as u32,
    KeyCode::Digit6 => '6' as u32,
    KeyCode::Digit7 => '7' as u32,
    KeyCode::Digit8 => '8' as u32,
    KeyCode::Digit9 => '9' as u32,
    KeyCode::Comma => ',' as u32,
    KeyCode::Minus => '-' as u32,
    KeyCode::Period => '.' as u32,
    KeyCode::Space => ' ' as u32,
    KeyCode::Equal => '=' as u32,
    KeyCode::Semicolon => ';' as u32,
    KeyCode::Slash => '/' as u32,
    KeyCode::Backslash => '\\' as u32,
    KeyCode::Quote => '\'' as u32,
    KeyCode::Backquote => '`' as u32,
    KeyCode::BracketLeft => '[' as u32,
    KeyCode::BracketRight => ']' as u32,
    k => {
      if let Some(gdk_key) = key_to_raw_key(k) {
        *gdk_key
      } else {
        dbg!("Cannot map key {:?}", k);
        return None;
      }
    }
  };

  let accel_mods = modifiers_to_gdk_modifier_type(key.mods);
  Some((accel_key, accel_mods))
}

fn modifiers_to_gdk_modifier_type(modifiers: ModifiersState) -> gdk::ModifierType {
  let mut result = gdk::ModifierType::empty();

  result.set(gdk::ModifierType::MOD1_MASK, modifiers.alt_key());
  result.set(gdk::ModifierType::CONTROL_MASK, modifiers.control_key());
  result.set(gdk::ModifierType::SHIFT_MASK, modifiers.shift_key());
  result.set(gdk::ModifierType::META_MASK, modifiers.super_key());

  result
}
