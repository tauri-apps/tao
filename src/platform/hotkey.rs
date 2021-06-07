#![cfg(any(
  target_os = "windows",
  target_os = "macos",
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

// TODO: Maybe merge this with `modifier_supplement` if the two are indeed supported on the same
// set of platforms
use crate::{hotkey::HotKey, platform_impl::GlobalAccelerator};

pub trait HotKeyExtGlobalAccelerator {
  fn register_global(self);
}

impl HotKeyExtGlobalAccelerator for HotKey {
  fn register_global(self) {
    if let Some(all_keycode) = self.key.to_keycode() {
      println!("all_keycode {:?}", all_keycode);
      // by example, on macOS `cmd` represent 2 scancode (left and right), so we need to register 2 different
      // hotkey with the same `id` if we want to get bot to work
      for keycode in all_keycode {
        GlobalAccelerator::new(self.mods.into(), keycode);
      }
    }
  }
}
