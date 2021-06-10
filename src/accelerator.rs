//! The Accelerator struct and associated types.

use crate::{
  keyboard::{Key, ModifiersState, NativeKeyCode},
};
use std::{
  borrow::Borrow,
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Accelerator {
  pub(crate) mods: ModifiersState,
  pub(crate) key: Key,
}

impl Accelerator {
  pub fn new(mods: impl Into<Option<ModifiersState>>, key: impl Into<Key>) -> Self {
    Self {
      mods: mods.into().unwrap_or(ModifiersState::empty()),
      key: key.into(),
    }
  }

  pub fn from_str(accelerator_string: &str) -> Accelerator {
    parse_accelerator(accelerator_string)
  }

  pub fn id(self) -> u16 {
    hash_accelerator_to_u16(self)
  }

  /// Returns `true` if this [`Key`] and [`ModifiersState`] matches this `Accelerator`.
  ///
  /// [`Key`]: Key
  /// [`ModifiersState`]: crate::keyboard::ModifiersState
  pub fn matches(&self, modifiers: impl Borrow<ModifiersState>, key: impl Borrow<Key>) -> bool {
    // Should be a const but const bit_or doesn't work here.
    let base_mods =
      ModifiersState::SHIFT | ModifiersState::CONTROL | ModifiersState::ALT | ModifiersState::SUPER;
    let modifiers = modifiers.borrow();
    let key = key.borrow();
    self.mods == *modifiers & base_mods && self.key == *key
  }
}

/// Represents the platform-agnostic keyboard modifiers, for command handling.
///
/// **This does one thing: it allows specifying accelerators that use the Command key
/// on macOS, but use the Ctrl key on other platforms.**
#[derive(Debug, Clone, Copy)]
pub enum SysMods {
  None,
  Shift,
  /// Command on macOS, and Ctrl on windows/linux
  Cmd,
  /// Command + Alt on macOS, Ctrl + Alt on windows/linux
  AltCmd,
  /// Command + Shift on macOS, Ctrl + Shift on windows/linux
  CmdShift,
  /// Command + Alt + Shift on macOS, Ctrl + Alt + Shift on windows/linux
  AltCmdShift,
}

/// Represents the active modifier keys.
///
/// This is intended to be clearer than [`ModifiersState`], when describing accelerators.
///
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum RawMods {
  None,
  Alt,
  Ctrl,
  Meta,
  Shift,
  AltCtrl,
  AltMeta,
  AltShift,
  CtrlShift,
  CtrlMeta,
  MetaShift,
  AltCtrlMeta,
  AltCtrlShift,
  AltMetaShift,
  CtrlMetaShift,
  AltCtrlMetaShift,
}

impl std::cmp::PartialEq<ModifiersState> for RawMods {
  fn eq(&self, other: &ModifiersState) -> bool {
    let mods: ModifiersState = (*self).into();
    mods == *other
  }
}

impl std::cmp::PartialEq<RawMods> for ModifiersState {
  fn eq(&self, other: &RawMods) -> bool {
    other == self
  }
}

impl std::cmp::PartialEq<ModifiersState> for SysMods {
  fn eq(&self, other: &ModifiersState) -> bool {
    let mods: RawMods = (*self).into();
    mods == *other
  }
}

impl std::cmp::PartialEq<SysMods> for ModifiersState {
  fn eq(&self, other: &SysMods) -> bool {
    let other: RawMods = (*other).into();
    &other == self
  }
}

// we do this so that Accelerator::new can accept `None` as an initial argument.
impl From<SysMods> for Option<ModifiersState> {
  fn from(src: SysMods) -> Option<ModifiersState> {
    Some(src.into())
  }
}

impl From<RawMods> for Option<ModifiersState> {
  fn from(src: RawMods) -> Option<ModifiersState> {
    Some(src.into())
  }
}

impl From<RawMods> for ModifiersState {
  fn from(src: RawMods) -> ModifiersState {
    let (alt, ctrl, meta, shift) = match src {
      RawMods::None => (false, false, false, false),
      RawMods::Alt => (true, false, false, false),
      RawMods::Ctrl => (false, true, false, false),
      RawMods::Meta => (false, false, true, false),
      RawMods::Shift => (false, false, false, true),
      RawMods::AltCtrl => (true, true, false, false),
      RawMods::AltMeta => (true, false, true, false),
      RawMods::AltShift => (true, false, false, true),
      RawMods::CtrlMeta => (false, true, true, false),
      RawMods::CtrlShift => (false, true, false, true),
      RawMods::MetaShift => (false, false, true, true),
      RawMods::AltCtrlMeta => (true, true, true, false),
      RawMods::AltMetaShift => (true, false, true, true),
      RawMods::AltCtrlShift => (true, true, false, true),
      RawMods::CtrlMetaShift => (false, true, true, true),
      RawMods::AltCtrlMetaShift => (true, true, true, true),
    };
    let mut mods = ModifiersState::empty();
    mods.set(ModifiersState::ALT, alt);
    mods.set(ModifiersState::CONTROL, ctrl);
    mods.set(ModifiersState::SUPER, meta);
    mods.set(ModifiersState::SHIFT, shift);
    mods
  }
}

impl From<SysMods> for ModifiersState {
  fn from(src: SysMods) -> ModifiersState {
    let (alt, ctrl, meta, shift) = match src {
      SysMods::None => (false, false, false, false),
      SysMods::Shift => (false, false, false, true),

      #[cfg(target_os = "macos")]
      SysMods::AltCmd => (true, false, true, false),
      #[cfg(not(target_os = "macos"))]
      SysMods::AltCmd => (true, true, false, false),

      #[cfg(target_os = "macos")]
      SysMods::AltCmdShift => (true, false, true, true),
      #[cfg(not(target_os = "macos"))]
      SysMods::AltCmdShift => (true, true, false, true),

      #[cfg(target_os = "macos")]
      SysMods::Cmd => (false, false, true, false),
      #[cfg(not(target_os = "macos"))]
      SysMods::Cmd => (false, true, false, false),

      #[cfg(target_os = "macos")]
      SysMods::CmdShift => (false, false, true, true),
      #[cfg(not(target_os = "macos"))]
      SysMods::CmdShift => (false, true, false, true),
    };
    let mut mods = ModifiersState::empty();
    mods.set(ModifiersState::ALT, alt);
    mods.set(ModifiersState::CONTROL, ctrl);
    mods.set(ModifiersState::SUPER, meta);
    mods.set(ModifiersState::SHIFT, shift);
    mods
  }
}

impl From<SysMods> for RawMods {
  fn from(src: SysMods) -> RawMods {
    #[cfg(target_os = "macos")]
    match src {
      SysMods::None => RawMods::None,
      SysMods::Shift => RawMods::Shift,
      SysMods::Cmd => RawMods::Meta,
      SysMods::AltCmd => RawMods::AltMeta,
      SysMods::CmdShift => RawMods::MetaShift,
      SysMods::AltCmdShift => RawMods::AltMetaShift,
    }
    #[cfg(not(target_os = "macos"))]
    match src {
      SysMods::None => RawMods::None,
      SysMods::Shift => RawMods::Shift,
      SysMods::Cmd => RawMods::Ctrl,
      SysMods::AltCmd => RawMods::AltCtrl,
      SysMods::CmdShift => RawMods::CtrlShift,
      SysMods::AltCmdShift => RawMods::AltCtrlShift,
    }
  }
}

fn parse_accelerator(accelerator_string: &str) -> Accelerator {
  let mut mods = ModifiersState::empty();
  let mut key = Key::Unidentified(NativeKeyCode::Unidentified);

  for raw in accelerator_string.to_uppercase().split('+') {
    let token = raw.trim().to_string();
    if token.is_empty() {
      continue;
    }

    match token.as_str() {
      "OPTION" | "ALT" => {
        mods.set(ModifiersState::ALT, true);
        continue;
      }
      "CONTROL" | "CTRL" => {
        mods.set(ModifiersState::CONTROL, true);
        continue;
      }
      "COMMAND" | "CMD" | "SUPER" => {
        mods.set(ModifiersState::SUPER, true);
        continue;
      }
      "SHIFT" => {
        mods.set(ModifiersState::SHIFT, true);
        continue;
      }
      "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL" | "CMDORCONTROL" => {
        #[cfg(target_os = "macos")]
        mods.set(ModifiersState::SUPER, true);
        #[cfg(not(target_os = "macos"))]
        mods.set(ModifiersState::CONTROL, true);
        continue;
      }
      _ => {}
    }

    key = match token.to_uppercase().as_str() {
      // alphabets
      "A" => Key::Character("a".into()),
      "B" => Key::Character("b".into()),
      "C" => Key::Character("c".into()),
      "D" => Key::Character("d".into()),
      "E" => Key::Character("e".into()),
      "F" => Key::Character("f".into()),
      "G" => Key::Character("g".into()),
      "H" => Key::Character("h".into()),
      "I" => Key::Character("i".into()),
      "J" => Key::Character("j".into()),
      "K" => Key::Character("k".into()),
      "L" => Key::Character("l".into()),
      "M" => Key::Character("m".into()),
      "N" => Key::Character("n".into()),
      "O" => Key::Character("o".into()),
      "P" => Key::Character("p".into()),
      "Q" => Key::Character("q".into()),
      "R" => Key::Character("r".into()),
      "S" => Key::Character("s".into()),
      "T" => Key::Character("t".into()),
      "U" => Key::Character("u".into()),
      "V" => Key::Character("v".into()),
      "W" => Key::Character("w".into()),
      "X" => Key::Character("x".into()),
      "Y" => Key::Character("y".into()),
      "Z" => Key::Character("z".into()),
      // numpad
      "0" => Key::Character("0".into()),
      "1" => Key::Character("1".into()),
      "2" => Key::Character("2".into()),
      "3" => Key::Character("3".into()),
      "4" => Key::Character("4".into()),
      "5" => Key::Character("5".into()),
      "6" => Key::Character("6".into()),
      "7" => Key::Character("7".into()),
      "8" => Key::Character("8".into()),
      "9" => Key::Character("9".into()),
      // shortcuts
      "FN" => Key::Fn,
      "BACKSPACE" => Key::Backspace,
      "TAB" => Key::Tab,
      "ENTER" => Key::Enter,
      "CAPSLOCK" => Key::CapsLock,
      "ESCAPE" => Key::Escape,
      "SPACE" => Key::Space,
      "PAGEUP" => Key::PageUp,
      "PAGEDOWN" => Key::PageDown,
      "END" => Key::End,
      "HOME" => Key::Home,
      "PRINTSCREEN" => Key::PrintScreen,
      "INSERT" => Key::Insert,
      "CLEAR" => Key::Clear,
      "DELETE" => Key::Delete,
      "SCROLLLOCK" => Key::ScrollLock,
      "HELP" => Key::Help,
      "NUMLOCK" => Key::NumLock,
      "VOLUMEMUTE" => Key::AudioVolumeMute,
      "VOLUMEDOWN" => Key::AudioVolumeDown,
      "VOLUMEUP" => Key::AudioVolumeUp,
      "MEDIANEXTTRACK" => Key::MediaTrackNext,
      "MEDIAPREVIOUSTRACK" => Key::MediaTrackPrevious,
      "MEDIAPLAYPAUSE" => Key::MediaPlayPause,
      "LAUNCHMAIL" => Key::LaunchMail,
      // f key's
      "F1" => Key::F1,
      "F2" => Key::F2,
      "F3" => Key::F3,
      "F4" => Key::F4,
      "F5" => Key::F5,
      "F6" => Key::F6,
      "F7" => Key::F7,
      "F8" => Key::F8,
      "F9" => Key::F9,
      "F10" => Key::F10,
      "F11" => Key::F11,
      "F12" => Key::F12,
      "F13" => Key::F13,
      "F14" => Key::F14,
      "F15" => Key::F15,
      "F16" => Key::F16,
      "F17" => Key::F17,
      "F18" => Key::F18,
      "F19" => Key::F19,
      // arrows
      "LEFT" => Key::ArrowLeft,
      "RIGHT" => Key::ArrowRight,
      "UP" => Key::ArrowUp,
      "DOWN" => Key::ArrowDown,
      _ => Key::Unidentified(NativeKeyCode::Unidentified),
    };
  }

  Accelerator {
    key,
    mods: mods.into(),
  }
}

fn hash_accelerator_to_u16(hotkey: Accelerator) -> u16 {
  let mut s = DefaultHasher::new();
  hotkey.hash(&mut s);
  s.finish() as u16
}
