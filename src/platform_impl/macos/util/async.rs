// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// TODO: Migrate this to https://github.com/rust-windowing/winit/pull/4589

use std::ops::Deref;

use dispatch2::DispatchQueue;
use objc2::{rc::Retained, DefinedClass, MainThreadMarker};
use objc2_app_kit::{NSScreen, NSWindowStyleMask};

use crate::platform_impl::platform::window_delegate::{WindowDelegate, DEFAULT_STANDARD_FRAME};

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
  window_delegate: Retained<WindowDelegate>,
  is_zoomed: bool,
  maximized: bool,
) {
  let window_delegate = MainThreadSafe(window_delegate);
  DispatchQueue::main().exec_async(move || {
    if !is_zoomed {
      window_delegate
        .ivars()
        .standard_frame
        .set(Some(window_delegate.window().frame()));
    }
    window_delegate.ivars().maximized.set(maximized);
    if window_delegate.ivars().fullscreen.borrow().is_some() {
      // Apply the requested state when native fullscreen ends.
      return;
    }
    let standard_frame = window_delegate
      .ivars()
      .standard_frame
      .get()
      .unwrap_or(DEFAULT_STANDARD_FRAME);

    let required = NSWindowStyleMask::Resizable | NSWindowStyleMask::Titled;
    if window_delegate.window().styleMask().contains(required) {
      window_delegate.window().zoom(None);
    } else {
      let frame = if maximized {
        let mtm = MainThreadMarker::from(window_delegate.window());
        NSScreen::mainScreen(mtm).unwrap().visibleFrame()
      } else {
        standard_frame
      };
      window_delegate
        .window()
        .setFrame_display_animate(frame, false, true);
    }
  });
}
