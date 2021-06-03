use crate::{
  event::{ElementState, KeyEvent},
  keyboard::{Key, KeyLocation, ModifiersState, NativeKeyCode},
};
use gdk::{keys::constants::*, EventKey, ModifierType};
use std::ffi::c_void;
use std::ops::Index;
use std::os::raw::{c_int, c_uint};
use std::ptr;
use std::slice;

pub type RawKey = gdk::keys::Key;

#[allow(clippy::just_underscores_and_digits, non_upper_case_globals)]
pub fn raw_key_to_key(gdk_key: RawKey) -> Option<Key<'static>> {
  let unicode = gdk_key.to_unicode();

  let key = match gdk_key {
    Escape => Some(Key::Escape),
    BackSpace => Some(Key::Backspace),
    Tab | ISO_Left_Tab => Some(Key::Tab),
    Return => Some(Key::Enter),
    Control_L | Control_R => Some(Key::Control),
    Alt_L | Alt_R => Some(Key::Alt),
    Shift_L | Shift_R => Some(Key::Shift),
    // TODO: investigate mapping. Map Meta_[LR]?
    Super_L | Super_R => Some(Key::Super),
    Caps_Lock => Some(Key::CapsLock),
    F1 => Some(Key::F1),
    F2 => Some(Key::F2),
    F3 => Some(Key::F3),
    F4 => Some(Key::F4),
    F5 => Some(Key::F5),
    F6 => Some(Key::F6),
    F7 => Some(Key::F7),
    F8 => Some(Key::F8),
    F9 => Some(Key::F9),
    F10 => Some(Key::F10),
    F11 => Some(Key::F11),
    F12 => Some(Key::F12),

    Print => Some(Key::PrintScreen),
    Scroll_Lock => Some(Key::ScrollLock),
    // Pause/Break not audio.
    Pause => Some(Key::Pause),

    Insert => Some(Key::Insert),
    Delete => Some(Key::Delete),
    Home => Some(Key::Home),
    End => Some(Key::End),
    Page_Up => Some(Key::PageUp),
    Page_Down => Some(Key::PageDown),
    Num_Lock => Some(Key::NumLock),

    Up => Some(Key::ArrowUp),
    Down => Some(Key::ArrowDown),
    Left => Some(Key::ArrowLeft),
    Right => Some(Key::ArrowRight),
    Clear => Some(Key::Clear),

    Menu => Some(Key::ContextMenu),
    WakeUp => Some(Key::WakeUp),
    Launch0 => Some(Key::LaunchApplication1),
    Launch1 => Some(Key::LaunchApplication2),
    ISO_Level3_Shift => Some(Key::AltGraph),

    KP_Begin => Some(Key::Clear),
    KP_Delete => Some(Key::Delete),
    KP_Down => Some(Key::ArrowDown),
    KP_End => Some(Key::End),
    KP_Enter => Some(Key::Enter),
    KP_F1 => Some(Key::F1),
    KP_F2 => Some(Key::F2),
    KP_F3 => Some(Key::F3),
    KP_F4 => Some(Key::F4),
    KP_Home => Some(Key::Home),
    KP_Insert => Some(Key::Insert),
    KP_Left => Some(Key::ArrowLeft),
    KP_Page_Down => Some(Key::PageDown),
    KP_Page_Up => Some(Key::PageUp),
    KP_Right => Some(Key::ArrowRight),
    // KP_Separator? What does it map to?
    KP_Tab => Some(Key::Tab),
    KP_Up => Some(Key::ArrowUp),
    // TODO: more mappings (media etc)
    _ => return None,
  };

  key
}

pub fn raw_key_to_location(raw: RawKey) -> KeyLocation {
  match raw {
    Control_L | Shift_L | Alt_L | Super_L | Meta_L => KeyLocation::Left,
    Control_R | Shift_R | Alt_R | Super_R | Meta_R => KeyLocation::Right,
    KP_0 | KP_1 | KP_2 | KP_3 | KP_4 | KP_5 | KP_6 | KP_7 | KP_8 | KP_9 | KP_Add | KP_Begin
    | KP_Decimal | KP_Delete | KP_Divide | KP_Down | KP_End | KP_Enter | KP_Equal | KP_F1
    | KP_F2 | KP_F3 | KP_F4 | KP_Home | KP_Insert | KP_Left | KP_Multiply | KP_Page_Down
    | KP_Page_Up | KP_Right | KP_Separator | KP_Space | KP_Subtract | KP_Tab | KP_Up => {
      KeyLocation::Numpad
    }
    _ => KeyLocation::Standard,
  }
}

const MODIFIER_MAP: &[(ModifierType, ModifiersState)] = &[
  (ModifierType::SHIFT_MASK, ModifiersState::SHIFT),
  (ModifierType::MOD1_MASK, ModifiersState::ALT),
  (ModifierType::CONTROL_MASK, ModifiersState::CONTROL),
  (ModifierType::SUPER_MASK, ModifiersState::SUPER),
];

pub(crate) fn get_modifiers(modifiers: ModifierType) -> ModifiersState {
  let mut result = ModifiersState::empty();
  for &(gdk_mod, modifier) in MODIFIER_MAP {
    if modifiers.contains(gdk_mod) {
      result |= modifier;
    }
  }
  result
}

pub fn make_key_event(key: &EventKey, repeat: bool, state: ElementState) -> Option<KeyEvent> {
  let keyval = key.get_keyval();
  let hardware_keycode = key.get_hardware_keycode();
  let unicode = gdk::keys::keyval_to_unicode(*keyval);
  let keycode = hardware_keycode_to_keyval(hardware_keycode).unwrap_or_else(|| keyval.clone());

  let mods = get_modifiers(key.get_state());
  let key = raw_key_to_key(keyval).unwrap_or_else(|| {
    if let Some(char) = unicode {
      if char >= ' ' && char != '\x7f' {
        Key::Character(insert_or_get_key_str(char.to_string()))
      } else {
        Key::Unidentified(NativeKeyCode::Gtk(hardware_keycode))
      }
    } else {
      Key::Unidentified(NativeKeyCode::Gtk(hardware_keycode))
    }
  });
  //let code = hardware_keycode_to_code(hardware_keycode);
  //let location = raw_key_to_location(keycode);
  let is_composing = false;

  None
}

// maybe create caching like macos
fn insert_or_get_key_str(string: String) -> &'static str {
  let static_str = Box::leak(string.into_boxed_str());
  static_str
}

/// Map a hardware keycode to a keyval by performing a lookup in the keymap and finding the
/// keyval with the lowest group and level
fn hardware_keycode_to_keyval(keycode: u16) -> Option<RawKey> {
  use glib::translate::FromGlib;
  unsafe {
    let keymap = gdk_sys::gdk_keymap_get_default();

    let mut nkeys = 0;
    let mut keys: *mut gdk_sys::GdkKeymapKey = ptr::null_mut();
    let mut keyvals: *mut c_uint = ptr::null_mut();

    // call into gdk to retrieve the keyvals and keymap keys
    gdk_sys::gdk_keymap_get_entries_for_keycode(
      keymap,
      c_uint::from(keycode),
      &mut keys as *mut *mut gdk_sys::GdkKeymapKey,
      &mut keyvals as *mut *mut c_uint,
      &mut nkeys as *mut c_int,
    );

    if nkeys > 0 {
      let keyvals_slice = slice::from_raw_parts(keyvals, nkeys as usize);
      let keys_slice = slice::from_raw_parts(keys, nkeys as usize);

      let resolved_keyval = keys_slice.iter().enumerate().find_map(|(id, gdk_keymap)| {
        if gdk_keymap.group == 0 && gdk_keymap.level == 0 {
          Some(RawKey::from_glib(keyvals_slice[id]))
        } else {
          None
        }
      });

      // notify glib to free the allocated arrays
      glib_sys::g_free(keyvals as *mut c_void);
      glib_sys::g_free(keys as *mut c_void);
    }
  }
  None
}
