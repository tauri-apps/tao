// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(any(
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

mod clipboard;
mod device;
mod event_loop;
mod global_shortcut;
mod icon;
mod keyboard;
mod keycode;
mod menu;
mod monitor;
#[cfg(feature = "tray")]
mod system_tray;
mod window;
pub mod x11;

#[cfg(feature = "tray")]
pub use self::system_tray::{SystemTray, SystemTrayBuilder};
pub use self::{
  clipboard::Clipboard,
  global_shortcut::{GlobalShortcut, ShortcutManager},
  keycode::{keycode_from_scancode, keycode_to_scancode},
  menu::{Menu, MenuItemAttributes},
};
pub use event_loop::{EventLoop, EventLoopProxy, EventLoopWindowTarget};
pub use icon::PlatformIcon;
pub use monitor::{MonitorHandle, VideoMode};
pub use window::{hit_test, Window, WindowId};

use crate::{event::DeviceId as RootDeviceId, keyboard::Key};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct KeyEventExtra {
  pub text_with_all_modifiers: Option<&'static str>,
  pub key_without_modifiers: Key<'static>,
}

#[non_exhaustive]
#[derive(Clone)]
pub enum Parent {
  None,
  ChildOf(gtk::ApplicationWindow),
}

impl Default for Parent {
  fn default() -> Self {
    Parent::None
  }
}

#[derive(Clone)]
pub struct PlatformSpecificWindowBuilderAttributes {
  pub parent: Parent,
  pub skip_taskbar: bool,
  pub auto_transparent: bool,
  pub double_buffered: bool,
  pub app_paintable: bool,
  pub rgba_visual: bool,
  pub cursor_moved: bool,
}

impl Default for PlatformSpecificWindowBuilderAttributes {
  fn default() -> Self {
    Self {
      parent: Default::default(),
      skip_taskbar: Default::default(),
      auto_transparent: true,
      double_buffered: true,
      app_paintable: false,
      rgba_visual: false,
      cursor_moved: true,
    }
  }
}

unsafe impl Send for PlatformSpecificWindowBuilderAttributes {}
unsafe impl Sync for PlatformSpecificWindowBuilderAttributes {}

#[derive(Debug)]
pub enum OsError {
  IoError(std::io::Error),
  PngEncodingError(png::EncodingError),
}

impl std::fmt::Display for OsError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    match self {
      OsError::IoError(e) => f.pad(&e.to_string()),
      OsError::PngEncodingError(e) => f.pad(&e.to_string()),
    }
  }
}

impl From<std::io::Error> for OsError {
  fn from(value: std::io::Error) -> OsError {
    OsError::IoError(value)
  }
}

impl From<png::EncodingError> for OsError {
  fn from(value: png::EncodingError) -> OsError {
    OsError::PngEncodingError(value)
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(usize);

impl DeviceId {
  pub unsafe fn dummy() -> Self {
    Self(0)
  }
}

// FIXME: currently we use a dummy device id, find if we can get device id from gtk
pub(crate) const DEVICE_ID: RootDeviceId = RootDeviceId(DeviceId(0));
