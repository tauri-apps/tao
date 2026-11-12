// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  ops::Deref,
  sync::{Mutex, Weak},
};

use dispatch2::DispatchQueue;
use objc2::Message;
use objc2_app_kit::{NSView, NSWindow, NSWindowStyleMask};

use crate::platform_impl::platform::{ffi, window::SharedState};

pub fn is_main_thread() -> bool {
  unsafe { msg_send!(class!(NSThread), isMainThread) }
}

// Unsafe wrapper type that allows us to dispatch things that aren't Send.
// This should *only* be used to dispatch to the main queue.
// While it is indeed not guaranteed that these types can safely be sent to
// other threads, we know that they're safe to use on the main thread.
struct MainThreadSafe<T>(T);

unsafe impl<T> Send for MainThreadSafe<T> {}

impl<T> Deref for MainThreadSafe<T> {
  type Target = T;
  fn deref(&self) -> &T {
    &self.0
  }
}

unsafe fn set_style_mask(ns_window: &NSWindow, ns_view: &NSView, mask: NSWindowStyleMask) {
  ns_window.setStyleMask(mask);
  // If we don't do this, key handling will break
  // (at least until the window is clicked again/etc.)
  ns_window.makeFirstResponder(Some(ns_view));
}

// Defer fullscreen changes until the caller has released the shared-state lock;
// changing styles or toggling fullscreen can invoke window delegate callbacks.
pub unsafe fn toggle_full_screen_async(
  ns_window: &NSWindow,
  ns_view: &NSView,
  not_fullscreen: bool,
  shared_state: Weak<Mutex<SharedState>>,
) {
  let ns_window = MainThreadSafe(ns_window.retain());
  let ns_view = MainThreadSafe(ns_view.retain());
  let shared_state = MainThreadSafe(shared_state);
  DispatchQueue::main().exec_async(move || {
    // `toggleFullScreen` doesn't work if the `StyleMask` is none, so we
    // set a normal style temporarily. The previous state will be
    // restored in `WindowDelegate::window_did_exit_fullscreen`.
    if not_fullscreen {
      let curr_mask = ns_window.styleMask();
      let required = NSWindowStyleMask::Titled | NSWindowStyleMask::Resizable;
      if !curr_mask.contains(required) {
        set_style_mask(&ns_window, &ns_view, required);
        if let Some(shared_state) = shared_state.upgrade() {
          trace!("Locked shared state in `toggle_full_screen_callback`");
          let mut shared_state_lock = shared_state.lock().unwrap();
          shared_state_lock.saved_style = Some(curr_mask);
          trace!("Unlocked shared state in `toggle_full_screen_callback`");
        }
      }
    }
    // Window level must be restored from `CGShieldingWindowLevel()
    // + 1` back to normal in order for `toggleFullScreen` to do
    // anything
    ns_window.setLevel(0);
    ns_window.toggleFullScreen(None);
  });
}

pub unsafe fn restore_display_mode_async(ns_screen: u32) {
  DispatchQueue::main().exec_async(move || {
    ffi::CGRestorePermanentDisplayConfiguration();
    assert_eq!(ffi::CGDisplayRelease(ns_screen), ffi::kCGErrorSuccess);
  });
}
