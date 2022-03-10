#![cfg(any(
  target_os = "windows",
  target_os = "macos",
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

//! **UNSTABLE** -- The `GlobalShortcut` struct and associated types.
//!
//! ## Platform-specific
//!
//! - **Linux**: Only works on x11. See [#331](https://github.com/tauri-apps/tao/issues/331) for more information.
//!
//! ```rust,ignore
//! let mut hotkey_manager = ShortcutManager::new(&event_loop);
//! let accelerator = Accelerator::new(SysMods::Shift, KeyCode::ArrowUp);
//! let global_shortcut = hotkey_manager.register(accelerator)?;
//! ```

use crate::{
  accelerator::Accelerator,
  event_loop::EventLoopWindowTarget,
  platform_impl::{
    GlobalShortcut as GlobalShortcutPlatform, ShortcutManager as ShortcutManagerPlatform,
  },
};
use std::{error, fmt};

/// Describes a global keyboard shortcut.
#[derive(Debug, Clone)]
pub struct GlobalShortcut(pub(crate) GlobalShortcutPlatform);

/// Object that allows you to manage a `GlobalShortcut`.
#[derive(Debug)]
pub struct ShortcutManager {
  registered_hotkeys: Vec<Accelerator>,
  p: ShortcutManagerPlatform,
}

impl ShortcutManager {
  /// Creates a new shortcut manager instance connected to the event loop.
  pub fn new<T: 'static>(event_loop: &EventLoopWindowTarget<T>) -> ShortcutManager {
    ShortcutManager {
      p: ShortcutManagerPlatform::new(event_loop),
      registered_hotkeys: Vec::new(),
    }
  }

  /// Whether the application has registered this `Accelerator`.
  pub fn is_registered(&self, accelerator: &Accelerator) -> bool {
    self.registered_hotkeys.contains(&Box::new(accelerator))
  }

  /// Register a global shortcut of `Accelerator` who trigger `GlobalShortcutEvent` in the event loop.
  pub fn register(
    &mut self,
    accelerator: Accelerator,
  ) -> Result<GlobalShortcut, ShortcutManagerError> {
    if self.is_registered(&accelerator) {
      return Err(ShortcutManagerError::AcceleratorAlreadyRegistered(
        accelerator,
      ));
    }
    self.registered_hotkeys.push(accelerator.clone());
    self.p.register(accelerator)
  }

  /// Unregister all `Accelerator` registered by the manager instance.
  pub fn unregister_all(&mut self) -> Result<(), ShortcutManagerError> {
    self.registered_hotkeys = Vec::new();
    self.p.unregister_all()
  }

  /// Unregister the provided `Accelerator`.
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

/// An error whose cause the `ShortcutManager` to fail.
#[non_exhaustive]
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
