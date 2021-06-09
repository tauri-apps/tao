//! The Accelerator struct and associated types.

use crate::keyboard::{Key, ModifiersState};
use std::{
  borrow::Borrow,
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Accelerator {
  pub(crate) mods: RawMods,
  pub(crate) key: Key,
}

impl Accelerator {
  pub fn new(mods: impl Into<Option<RawMods>>, key: impl Into<Key>) -> Self {
    Self {
      mods: mods.into().unwrap_or(RawMods::None),
      key: key.into(),
    }
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

// we do this so that Accelerator::new can accept `None` as an initial argument.
impl From<SysMods> for Option<RawMods> {
  fn from(src: SysMods) -> Option<RawMods> {
    Some(src.into())
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

fn hash_accelerator_to_u16(hotkey: Accelerator) -> u16 {
  let mut s = DefaultHasher::new();
  hotkey.hash(&mut s);
  s.finish() as u16
}
