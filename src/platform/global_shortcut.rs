#![cfg(any(
  target_os = "windows",
  target_os = "macos",
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

use crate::{
  accelerator::Accelerator,
  event_loop::EventLoopWindowTarget,
  platform_impl::{
    GlobalShortcut as GlobalShortcutPlatform, ShortcutManager as ShortcutManagerPlatform,
  },
};
use std::{error, fmt};
#[derive(Debug, Clone)]
pub struct GlobalShortcut(pub(crate) GlobalShortcutPlatform);

impl GlobalShortcut {
  pub fn unregister(&self) {
    self.0.unregister()
  }
}

#[derive(Debug, Clone)]
pub struct ShortcutManager {
  registered_hotkeys: Vec<Accelerator>,
  p: ShortcutManagerPlatform,
}

impl Default for ShortcutManager {
  fn default() -> Self {
    Self {
      registered_hotkeys: Vec::new(),
      p: ShortcutManagerPlatform::new(),
    }
  }
}

impl ShortcutManager {
  pub fn new() -> Self {
    Default::default()
  }

  pub fn is_registered(&self, _hotkey: &Accelerator) -> bool {
    //self.registered_hotkeys.contains(&Box::new(hotkey))
    false
  }

  pub fn register(
    &mut self,
    accelerator: Accelerator,
  ) -> Result<GlobalShortcut, ShortcutManagerError> {
    if self.is_registered(&accelerator) {
      return Err(ShortcutManagerError::AcceleratorAlreadyRegistered(
        accelerator,
      ));
    }
    self.p.register(accelerator)
  }

  pub fn unregister_all(&self) -> Result<(), ShortcutManagerError> {
    self.p.unregister_all()
  }

  pub fn connect_event_loop<T: 'static>(
    &mut self,
    _window_target: &EventLoopWindowTarget<T>,
  ) -> Result<(), ShortcutManagerError> {
    self.p.connect_event_loop(_window_target)
  }
}

#[derive(Debug)]
pub enum ShortcutManagerError {
  AcceleratorAlreadyRegistered(Accelerator),
  AcceleratorNotRegistered(Accelerator),
  InvalidAccelerator(String),
}

impl error::Error for ShortcutManagerError {}
impl fmt::Display for ShortcutManagerError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
    match self {
      ShortcutManagerError::AcceleratorAlreadyRegistered(e) => {
        f.pad(&format!("hotkey already registered: {:?}", e))
      }
      ShortcutManagerError::AcceleratorNotRegistered(e) => {
        f.pad(&format!("hotkey not registered: {:?}", e))
      }
      ShortcutManagerError::InvalidAccelerator(e) => e.fmt(f),
    }
  }
}
