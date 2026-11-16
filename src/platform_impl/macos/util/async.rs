// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  ops::Deref,
  sync::{Mutex, Weak},
};

use dispatch2::DispatchQueue;
use objc2::{MainThreadMarker, Message};
use objc2_app_kit::{NSScreen, NSView, NSWindow, NSWindowStyleMask};

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

// `toggleFullScreen` is thread-safe, but our additional logic to account for
// window styles isn't.
pub unsafe fn toggle_full_screen_async(
  window_delegate: Retained<WindowDelegate>,
  not_fullscreen: bool,
) {
  let window_delegate = MainThreadSafe(window_delegate);
  DispatchQueue::main().exec_async(move || {
    // `toggleFullScreen` doesn't work if the `StyleMask` is none, so we
    // set a normal style temporarily. The previous state will be
    // restored in `WindowDelegate::window_did_exit_fullscreen`.
    if not_fullscreen {
      let curr_mask = window_delegate.window().styleMask();
      let required = NSWindowStyleMask::Titled | NSWindowStyleMask::Resizable;
      if !curr_mask.contains(required) {
        window_delegate.set_style_mask(required);
        window_delegate.ivars().saved_style.set(Some(curr_mask));
      }
    }
    // Window level must be restored from `CGShieldingWindowLevel()
    // + 1` back to normal in order for `toggleFullScreen` to do
    // anything
    window_delegate.window().setLevel(0);
    window_delegate.window().toggleFullScreen(None);
  });
}

// `setMaximized` is not thread-safe
pub unsafe fn set_maximized_async(
  ns_window: &NSWindow,
  is_zoomed: bool,
  maximized: bool,
  shared_state: Weak<Mutex<SharedState>>,
) {
  let ns_window = MainThreadSafe(ns_window.retain());
  let shared_state = MainThreadSafe(shared_state);
  DispatchQueue::main().exec_async(move || {
    if let Some(shared_state) = shared_state.upgrade() {
      trace!("Locked shared state in `set_maximized`");
      let mut shared_state_lock = shared_state.lock().unwrap();

      // Save the standard frame sized if it is not zoomed
      if !is_zoomed {
        shared_state_lock.standard_frame = Some(NSWindow::frame(&ns_window));
      }

      shared_state_lock.maximized = maximized;

      let curr_mask = ns_window.styleMask();
      if shared_state_lock.fullscreen.is_some() {
        // Handle it in window_did_exit_fullscreen
        return;
      } else if curr_mask.contains(NSWindowStyleMask::Resizable)
        && curr_mask.contains(NSWindowStyleMask::Titled)
      {
        // Just use the native zoom if resizable
        ns_window.zoom(None);
      } else {
        // if it's not resizable, we set the frame directly
        let new_rect = if maximized {
          let mtm = MainThreadMarker::new_unchecked();
          let screen = NSScreen::mainScreen(mtm).unwrap();
          NSScreen::visibleFrame(&screen)
        } else {
          shared_state_lock.saved_standard_frame()
        };
        ns_window.setFrame_display_animate(new_rect, false, true);
      }

      trace!("Unlocked shared state in `set_maximized`");
    }
  });
}
