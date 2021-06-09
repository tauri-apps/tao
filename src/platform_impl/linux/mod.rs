// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(any(
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

mod event_loop;
mod global_shortcut;
mod keyboard;
mod menu;
mod monitor;
#[cfg(feature = "tray")]
mod system_tray;
mod window;

#[cfg(feature = "tray")]
pub use self::system_tray::{SystemTray, SystemTrayBuilder};
pub use self::{
  global_shortcut::{GlobalShortcut, ShortcutManager},
  menu::{Menu, MenuItemAttributes},
};
pub use event_loop::{EventLoop, EventLoopProxy, EventLoopWindowTarget};
pub use monitor::{MonitorHandle, VideoMode};
pub use window::{
  hit_test, PlatformIcon, PlatformSpecificWindowBuilderAttributes, Window, WindowId,
};

use crate::{
  event::KeyEvent, keyboard::Key, platform::modifier_supplement::KeyEventExtModifierSupplement,
};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct KeyEventExtra {
  pub text_with_all_modifiers: Option<String>,
  pub key_without_modifiers: Key,
}

impl KeyEventExtModifierSupplement for KeyEvent {
  fn text_with_all_modifiers(&self) -> Option<String> {
    self.platform_specific.text_with_all_modifiers.clone()
  }

  fn key_without_modifiers(&self) -> Key {
    self.platform_specific.key_without_modifiers.clone()
  }
}

#[derive(Debug, Clone)]
pub struct OsError;

impl std::fmt::Display for OsError {
  fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    Ok(())
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(usize);

impl DeviceId {
  pub unsafe fn dummy() -> Self {
    Self(0)
  }
}
