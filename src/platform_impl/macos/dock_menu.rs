// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use super::ffi::id;
use objc2::rc::Retained;
use objc2_app_kit::NSMenu;
use std::sync::Mutex;

/// The menu shown when the user right-clicks (or Control-clicks / long-presses)
/// the application's Dock icon. `None` means "no custom menu" — Cocoa falls
/// back to its default minimal menu (Show/Hide, Quit).
///
/// Stored as a raw pointer, not `Retained<NSMenu>`: Cocoa objects aren't
/// `Send`/`Sync`, and this is only ever touched from the main thread (same
/// invariant as the rest of this module's `id` usage, e.g. `app_state.rs`).
/// Retained manually (via `Retained::into_raw`/`from_raw`) rather than storing
/// a `Retained<NSMenu>` directly, because `applicationDockMenu:` must return a
/// value *synchronously* — there is no round-trip through `tao`'s event queue
/// like there is for e.g. `Event::Reopen`.
///
/// Wrapped in `MainThreadPtr` to satisfy `Sync` (raw pointers aren't
/// `Send`/`Sync` by default; see its doc comment for the actual invariant).
struct MainThreadPtr(Option<id>);
// SAFETY: `id` (an Objective-C object pointer) is only ever written or read
// from the main thread in this module — `set_dock_menu` takes a
// `Retained<NSMenu>`, which (like all Cocoa objects here) can only be
// constructed on the main thread, and `get_dock_menu` is only called from
// `application_dock_menu`, itself only ever invoked by Cocoa on the main
// thread. The `Mutex` exists solely to make the `static` legal, not for
// actual cross-thread coordination.
unsafe impl Send for MainThreadPtr {}
unsafe impl Sync for MainThreadPtr {}

static DOCK_MENU: Mutex<MainThreadPtr> = Mutex::new(MainThreadPtr(None));

/// Sets (or clears, with `None`) the application's Dock menu. Safe to call at
/// any point after the event loop has started, including from within a menu
/// item's own click handler (e.g. to rebuild the menu with updated
/// checkmarks) — the next right-click on the Dock icon picks up the change.
///
/// The previously registered menu (if any) is released.
pub fn set_dock_menu(menu: Option<Retained<NSMenu>>) {
  let new_ptr = menu.map(|m| Retained::into_raw(m) as id);
  let mut slot = DOCK_MENU.lock().unwrap();
  let old_ptr = std::mem::replace(&mut slot.0, new_ptr);
  if let Some(old_ptr) = old_ptr {
    // SAFETY: `old_ptr` was produced by a prior `Retained::into_raw` call in
    // this same function, so it's a valid, uniquely-owned +1 reference.
    unsafe { drop(Retained::from_raw(old_ptr as *mut NSMenu)) };
  }
}

/// Returns the currently registered Dock menu, if any, as a borrowed pointer
/// (still owned by this module — the caller must retain it before handing it
/// across an ownership boundary, e.g. back to Cocoa). Called from
/// `application_dock_menu` in `app_delegate.rs`.
pub(crate) fn get_dock_menu() -> Option<id> {
  DOCK_MENU.lock().unwrap().0
}
