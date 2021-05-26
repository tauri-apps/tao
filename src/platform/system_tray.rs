// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(any(
  target_os = "windows",
  target_os = "macos",
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

use crate::{platform_impl, menu::Menu, event_loop::EventLoopWindowTarget, error::OsError};
// TODO exhaustively match the targets
#[cfg(target_os = "linux")]
use std::path::PathBuf;

pub struct SystemTrayBuilder(platform_impl::SystemTrayBuilder);

impl SystemTrayBuilder {

  #[inline]
  #[cfg(not(target_os = "linux"))]
  pub fn new(icon: Vec<u8>, tray_menu: Menu) -> Self {
    Self(platform_impl::SystemTrayBuilder::new(icon, tray_menu.menu_platform))
  }

  #[inline]
  #[cfg(target_os = "linux")]
  pub fn new(icon: PathBuf, tray_menu: Menu) -> Self {
    Self(platform_impl::SystemTrayBuilder::new(icon, tray_menu.menu_platform))
  }

  #[inline]
  pub fn build<T: 'static>(
    self,
    _window_target: &EventLoopWindowTarget<T>,
  ) -> Result<platform_impl::SystemTray, OsError> {
    self.0.build(&_window_target)
  }
}
