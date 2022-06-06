use crate::{
  event::{ElementState, KeyboardInput},
  keyboard::{KeyCode, ModifiersState, VirtualKeyCode},
};
use gdk::{keys::constants::*, EventKey, ModifierType};
use std::{collections::HashSet, sync::Mutex};

pub type RawKey = gdk::keys::Key;

lazy_static! {
  static ref KEY_STRINGS: Mutex<HashSet<&'static str>> = Mutex::new(HashSet::new());
}

#[allow(clippy::just_underscores_and_digits, non_upper_case_globals)]
pub(crate) fn raw_key_to_key(gdk_key: RawKey) -> Option<VirtualKeyCode> {
  // TODO: more mappings
  match gdk_key {
    Escape => Some(VirtualKeyCode::Escape),
    // BackSpace => Some(VirtualKeyCode::Backspace),
    // Tab | ISO_Left_Tab => Some(VirtualKeyCode::Tab),
    // Return => Some(VirtualKeyCode::Enter),
    // Control_L | Control_R => Some(VirtualKeyCode::Control),
    // Alt_L | Alt_R => Some(VirtualKeyCode::Alt),
    // Shift_L | Shift_R => Some(VirtualKeyCode::Shift),
    // // TODO: investigate mapping. Map Meta_[LR]?
    // Super_L | Super_R => Some(VirtualKeyCode::Super),
    // Caps_Lock => Some(VirtualKeyCode::CapsLock),
    // F1 => Some(VirtualKeyCode::F1),
    // F2 => Some(VirtualKeyCode::F2),
    // F3 => Some(VirtualKeyCode::F3),
    // F4 => Some(VirtualKeyCode::F4),
    // F5 => Some(VirtualKeyCode::F5),
    // F6 => Some(VirtualKeyCode::F6),
    // F7 => Some(VirtualKeyCode::F7),
    // F8 => Some(VirtualKeyCode::F8),
    // F9 => Some(VirtualKeyCode::F9),
    // F10 => Some(VirtualKeyCode::F10),
    // F11 => Some(VirtualKeyCode::F11),
    // F12 => Some(VirtualKeyCode::F12),

    // Print => Some(VirtualKeyCode::PrintScreen),
    // Scroll_Lock => Some(VirtualKeyCode::ScrollLock),
    // // Pause/Break not audio.
    // Pause => Some(VirtualKeyCode::Pause),

    // Insert => Some(VirtualKeyCode::Insert),
    // Delete => Some(VirtualKeyCode::Delete),
    // Home => Some(VirtualKeyCode::Home),
    // End => Some(VirtualKeyCode::End),
    // Page_Up => Some(VirtualKeyCode::PageUp),
    // Page_Down => Some(VirtualKeyCode::PageDown),
    // Num_Lock => Some(VirtualKeyCode::NumLock),

    // Up => Some(VirtualKeyCode::ArrowUp),
    // Down => Some(VirtualKeyCode::ArrowDown),
    // Left => Some(VirtualKeyCode::ArrowLeft),
    // Right => Some(VirtualKeyCode::ArrowRight),
    // Clear => Some(VirtualKeyCode::Clear),

    // Menu => Some(VirtualKeyCode::ContextMenu),
    // WakeUp => Some(VirtualKeyCode::WakeUp),
    // Launch0 => Some(VirtualKeyCode::LaunchApplication1),
    // Launch1 => Some(VirtualKeyCode::LaunchApplication2),
    // ISO_Level3_Shift => Some(VirtualKeyCode::AltGraph),

    // KP_Begin => Some(VirtualKeyCode::Clear),
    // KP_Delete => Some(VirtualKeyCode::Delete),
    // KP_Down => Some(VirtualKeyCode::ArrowDown),
    // KP_End => Some(VirtualKeyCode::End),
    // KP_Enter => Some(VirtualKeyCode::Enter),
    // KP_F1 => Some(VirtualKeyCode::F1),
    // KP_F2 => Some(VirtualKeyCode::F2),
    // KP_F3 => Some(VirtualKeyCode::F3),
    // KP_F4 => Some(VirtualKeyCode::F4),
    // KP_Home => Some(VirtualKeyCode::Home),
    // KP_Insert => Some(VirtualKeyCode::Insert),
    // KP_Left => Some(VirtualKeyCode::ArrowLeft),
    // KP_Page_Down => Some(VirtualKeyCode::PageDown),
    // KP_Page_Up => Some(VirtualKeyCode::PageUp),
    // KP_Right => Some(VirtualKeyCode::ArrowRight),
    // // KP_Separator? What does it map to?
    // KP_Tab => Some(VirtualKeyCode::Tab),
    // KP_Up => Some(VirtualKeyCode::ArrowUp),
    _ => None,
  }
}

const MODIFIER_MAP: &[(ModifierType, ModifiersState)] = &[
  (ModifierType::SHIFT_MASK, ModifiersState::SHIFT),
  (ModifierType::MOD1_MASK, ModifiersState::ALT),
  (ModifierType::CONTROL_MASK, ModifiersState::CONTROL),
  (ModifierType::SUPER_MASK, ModifiersState::LOGO),
];

// we use the EventKey to extract the modifier mainly because
// we need to have the modifier before the second key is entered to follow
// other os' logic -- this way we can emit the new `ModifiersState` before
// we receive the next key, if needed the developer can update his local state.
pub(crate) fn get_modifiers(key: &EventKey) -> ModifiersState {
  let key_from_code = key.state();
  // start with empty state
  let mut result = ModifiersState::empty();
  // loop trough our modifier map
  for (gdk_mod, modifier) in MODIFIER_MAP {
    if key_from_code == *gdk_mod {
      result |= *modifier;
    }
  }
  result
}

pub(crate) fn make_key_event(key: &EventKey, state: ElementState) -> KeyboardInput {
  // a keycode (scancode in Windows) is a code that refers to a physical keyboard key.
  let scancode = key.hardware_keycode();
  // a keyval (keysym in X) is a "logical" key name, such as GDK_Enter, GDK_a, GDK_space, etc.
  // extract the logical key
  let logical_key = raw_key_to_key(key.keyval());

  #[allow(deprecated)]
  KeyboardInput {
    virtual_keycode: logical_key,
    scancode: scancode as u32,
    state,
    modifiers: get_modifiers(key),
  }
}

#[allow(non_upper_case_globals)]
pub fn key_to_raw_key(src: &KeyCode) -> Option<RawKey> {
  Some(match src {
    KeyCode::Escape => Escape,
    KeyCode::Backspace => BackSpace,

    KeyCode::Tab => Tab,
    KeyCode::Enter => Return,

    KeyCode::ControlLeft => Control_L,
    KeyCode::AltLeft => Alt_L,
    KeyCode::ShiftLeft => Shift_L,
    KeyCode::SuperLeft => Super_L,

    KeyCode::ControlRight => Control_R,
    KeyCode::AltRight => Alt_R,
    KeyCode::ShiftRight => Shift_R,
    KeyCode::SuperRight => Super_R,

    KeyCode::CapsLock => Caps_Lock,
    KeyCode::F1 => F1,
    KeyCode::F2 => F2,
    KeyCode::F3 => F3,
    KeyCode::F4 => F4,
    KeyCode::F5 => F5,
    KeyCode::F6 => F6,
    KeyCode::F7 => F7,
    KeyCode::F8 => F8,
    KeyCode::F9 => F9,
    KeyCode::F10 => F10,
    KeyCode::F11 => F11,
    KeyCode::F12 => F12,
    KeyCode::F13 => F13,
    KeyCode::F14 => F14,
    KeyCode::F15 => F15,
    KeyCode::F16 => F16,
    KeyCode::F17 => F17,
    KeyCode::F18 => F18,
    KeyCode::F19 => F19,
    KeyCode::F20 => F20,
    KeyCode::F21 => F21,
    KeyCode::F22 => F22,
    KeyCode::F23 => F23,
    KeyCode::F24 => F24,

    KeyCode::PrintScreen => Print,
    KeyCode::ScrollLock => Scroll_Lock,
    // Pause/Break not audio.
    KeyCode::Pause => Pause,

    KeyCode::Insert => Insert,
    KeyCode::Delete => Delete,
    KeyCode::Home => Home,
    KeyCode::End => End,
    KeyCode::PageUp => Page_Up,
    KeyCode::PageDown => Page_Down,

    KeyCode::NumLock => Num_Lock,

    KeyCode::ArrowUp => Up,
    KeyCode::ArrowDown => Down,
    KeyCode::ArrowLeft => Left,
    KeyCode::ArrowRight => Right,

    KeyCode::ContextMenu => Menu,
    KeyCode::WakeUp => WakeUp,
    _ => return None,
  })
}
