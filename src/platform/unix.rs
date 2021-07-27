// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(any(
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

pub use crate::platform_impl::{hit_test, Parent};
use crate::window::{Window, WindowBuilder};

/// Additional methods on `Window` that are specific to Unix.
pub trait WindowExtUnix {
  /// Returns the `ApplicatonWindow` from gtk crate that is used by this window.
  fn gtk_window(&self) -> &gtk::ApplicationWindow;

  /// Whether to show the window icon in the taskbar or not.
  fn set_skip_taskbar(&self, skip: bool);
}

impl WindowExtUnix for Window {
  fn gtk_window(&self) -> &gtk::ApplicationWindow {
    &self.window.window
  }

  fn set_skip_taskbar(&self, skip: bool) {
    self.window.set_skip_taskbar(skip);
  }
}

pub trait WindowBuilderExtUnix {
  /// Sets a parent to the window to be created.
  fn with_parent_window(self, parent: gtk::ApplicationWindow) -> WindowBuilder;
  /// Whether to create the window icon with the taskbar icon or not.
  fn with_skip_taskbar(self, skip: bool) -> WindowBuilder;
}

impl WindowBuilderExtUnix for WindowBuilder {
  fn with_parent_window(mut self, parent: gtk::ApplicationWindow) -> WindowBuilder {
    self.platform_specific.parent = Parent::ChildOf(parent);
    self
  }

  fn with_skip_taskbar(mut self, skip: bool) -> WindowBuilder {
    self.platform_specific.skip_taskbar = skip;
    self
  }
}
