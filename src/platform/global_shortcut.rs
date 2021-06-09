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

#[derive(Debug)]
pub struct ShortcutManager {
  registered_hotkeys: Vec<Accelerator>,
  p: ShortcutManagerPlatform,
}

impl ShortcutManager {
  pub fn new<T: 'static>(event_loop: &EventLoopWindowTarget<T>) -> ShortcutManager {
    ShortcutManager {
      p: ShortcutManagerPlatform::new(event_loop),
      registered_hotkeys: Vec::new(),
    }
  }

  pub fn is_registered(&self, accelerator: &Accelerator) -> bool {
    self.registered_hotkeys.contains(&Box::new(accelerator))
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

  pub fn unregister_all(&mut self) -> Result<(), ShortcutManagerError> {
    self.registered_hotkeys = Vec::new();
    self.p.unregister_all()
  }

  pub fn unregister(
    &mut self,
    global_shortcut: GlobalShortcut,
  ) -> Result<(), ShortcutManagerError> {
    self
      .registered_hotkeys
      .retain(|hotkey| hotkey.to_owned().id() != global_shortcut.0.id());
    self.p.unregister(global_shortcut)
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
