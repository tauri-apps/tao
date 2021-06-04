use super::KeyEventExtra;
use crate::{
  event::{ElementState, KeyEvent},
  keyboard::{Key, KeyCode, KeyLocation, ModifiersState, NativeKeyCode},
  platform::scancode::KeyCodeExtScancode,
};
use gdk::{keys::constants::*, EventKey};
use std::{
  collections::HashSet,
  ffi::c_void,
  os::raw::{c_int, c_uint},
  ptr, slice,
  sync::Mutex,
};

pub type RawKey = gdk::keys::Key;

lazy_static! {
  static ref KEY_STRINGS: Mutex<HashSet<&'static str>> = Mutex::new(HashSet::new());
}

fn insert_or_get_key_str(string: String) -> String {
  let mut string_set = KEY_STRINGS.lock().unwrap();
  if let Some(contained) = string_set.get(string.as_str()) {
    return contained.to_string();
  }
  let static_str = Box::leak(string.into_boxed_str());
  string_set.insert(static_str);
  static_str.to_string()
}

#[allow(clippy::just_underscores_and_digits, non_upper_case_globals)]
pub(crate) fn raw_key_to_key(gdk_key: RawKey) -> Option<Key> {
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

#[allow(clippy::just_underscores_and_digits, non_upper_case_globals)]
pub(crate) fn raw_key_to_location(raw: RawKey) -> KeyLocation {
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

const MODIFIER_MAP: &[(Key, ModifiersState)] = &[
  (Key::Shift, ModifiersState::SHIFT),
  (Key::Alt, ModifiersState::ALT),
  (Key::Control, ModifiersState::CONTROL),
  (Key::Super, ModifiersState::SUPER),
];

// we use the EventKey to extract the modifier mainly because
// we need to have the modifier before the second key is entered to follow
// other os' logic -- this way we can emit the new `ModifiersState` before
// we receive the next key, if needed the developer can update his local state.
pub(crate) fn get_modifiers(key: EventKey) -> ModifiersState {
  // a keycode (scancode in Windows) is a code that refers to a physical keyboard key.
  let scancode = key.get_hardware_keycode();
  // a keyval (keysym in X) is a "logical" key name, such as GDK_Enter, GDK_a, GDK_space, etc.
  let keyval = key.get_keyval();
  // unicode value
  let unicode = gdk::keys::keyval_to_unicode(*keyval);
  // translate to tao::keyboard::Key
  let key_from_code = raw_key_to_key(keyval).unwrap_or_else(|| {
    if let Some(key) = unicode {
      if key >= ' ' && key != '\x7f' {
        Key::Character(insert_or_get_key_str(key.to_string()))
      } else {
        Key::Unidentified(NativeKeyCode::Gtk(scancode))
      }
    } else {
      Key::Unidentified(NativeKeyCode::Gtk(scancode))
    }
  });
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

pub(crate) fn make_key_event(
  key: &EventKey,
  is_repeat: bool,
  key_override: Option<KeyCode>,
  state: ElementState,
) -> Option<KeyEvent> {
  // a keycode (scancode in Windows) is a code that refers to a physical keyboard key.
  let scancode = key.get_hardware_keycode();
  // a keyval (keysym in X) is a "logical" key name, such as GDK_Enter, GDK_a, GDK_space, etc.
  let keyval_without_modifiers = key.get_keyval();
  let keyval_with_modifiers =
    hardware_keycode_to_keyval(scancode).unwrap_or_else(|| keyval_without_modifiers.clone());
  // get unicode value, with and without modifiers
  let text_without_modifiers = gdk::keys::keyval_to_unicode(*keyval_with_modifiers.clone());
  let text_with_modifiers = gdk::keys::keyval_to_unicode(*keyval_without_modifiers);
  // get physical key from the scancode (keycode)
  let physical_key = key_override.unwrap_or_else(|| KeyCode::from_scancode(scancode as u32));

  // extract key without modifier
  let key_without_modifiers = raw_key_to_key(keyval_with_modifiers.clone()).unwrap_or_else(|| {
    if let Some(key) = text_without_modifiers {
      if key >= ' ' && key != '\x7f' {
        Key::Character(insert_or_get_key_str(key.to_string()))
      } else {
        Key::Unidentified(NativeKeyCode::Gtk(scancode))
      }
    } else {
      Key::Unidentified(NativeKeyCode::Gtk(scancode))
    }
  });

  // extract the logical key
  let logical_key = raw_key_to_key(keyval_without_modifiers.clone()).unwrap_or_else(|| {
    if let Some(key) = text_with_modifiers {
      if key >= ' ' && key != '\x7f' {
        Key::Character(insert_or_get_key_str(key.to_string()))
      } else {
        Key::Unidentified(NativeKeyCode::Gtk(scancode))
      }
    } else {
      Key::Unidentified(NativeKeyCode::Gtk(scancode))
    }
  });

  // make sure we have a valid key
  if !matches!(key_without_modifiers, Key::Unidentified(_)) {
    let location = raw_key_to_location(keyval_with_modifiers);
    let text_with_all_modifiers =
      text_without_modifiers.map(|text| insert_or_get_key_str(text.to_string()));
    return Some(KeyEvent {
      location,
      logical_key,
      physical_key,
      repeat: is_repeat,
      state,
      text: text_with_all_modifiers.clone(),
      platform_specific: KeyEventExtra {
        text_with_all_modifiers,
        key_without_modifiers,
      },
    });
  } else {
    println!("Couldn't get key from code: {:?}", physical_key);
  }
  None
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

      return resolved_keyval;
    }
  }
  None
}

#[allow(non_upper_case_globals)]
pub fn key_to_raw_key(src: &Key) -> Option<RawKey> {
  Some(match src {
    Key::Escape => Escape,
    Key::Backspace => BackSpace,

    Key::Tab => Tab,
    Key::Enter => Return,

    // Give "left" variants
    Key::Control => Control_L,
    Key::Alt => Alt_L,
    Key::Shift => Shift_L,
    Key::Super => Super_L,

    Key::CapsLock => Caps_Lock,
    Key::F1 => F1,
    Key::F2 => F2,
    Key::F3 => F3,
    Key::F4 => F4,
    Key::F5 => F5,
    Key::F6 => F6,
    Key::F7 => F7,
    Key::F8 => F8,
    Key::F9 => F9,
    Key::F10 => F10,
    Key::F11 => F11,
    Key::F12 => F12,

    Key::PrintScreen => Print,
    Key::ScrollLock => Scroll_Lock,
    // Pause/Break not audio.
    Key::Pause => Pause,

    Key::Insert => Insert,
    Key::Delete => Delete,
    Key::Home => Home,
    Key::End => End,
    Key::PageUp => Page_Up,
    Key::PageDown => Page_Down,

    Key::NumLock => Num_Lock,

    Key::ArrowUp => Up,
    Key::ArrowDown => Down,
    Key::ArrowLeft => Left,
    Key::ArrowRight => Right,

    Key::ContextMenu => Menu,
    Key::WakeUp => WakeUp,
    Key::LaunchApplication1 => Launch0,
    Key::LaunchApplication2 => Launch1,
    Key::AltGraph => ISO_Level3_Shift,
    // TODO: probably more
    _ => return None,
  })
}
