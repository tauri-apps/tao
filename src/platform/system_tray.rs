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

#[cfg(feature = "tray")]
use crate::{error::OsError, platform_impl};
use crate::{event_loop::EventLoopWindowTarget, menu::ContextMenu};
// TODO exhaustively match the targets
#[cfg(target_os = "linux")]
use std::path::PathBuf;
#[cfg(feature = "tray")]
pub struct SystemTrayBuilder(platform_impl::SystemTrayBuilder);

#[cfg(not(feature = "tray"))]
pub struct SystemTrayBuilder;

#[cfg(not(feature = "tray"))]
pub struct SystemTray;

#[cfg(feature = "tray")]
impl SystemTrayBuilder {
  #[inline]
  #[cfg(not(target_os = "linux"))]
  pub fn new(icon: Vec<u8>, tray_menu: Option<ContextMenu>) -> Self {
    Self(platform_impl::SystemTrayBuilder::new(
      icon,
      tray_menu.map(|m| m.0.menu_platform),
    ))
  }

  #[inline]
  #[cfg(target_os = "linux")]
  pub fn new(icon: PathBuf, tray_menu: Option<ContextMenu>) -> Self {
    Self(platform_impl::SystemTrayBuilder::new(
      icon,
      tray_menu.map(|m| m.0.menu_platform),
    ))
  }

  #[inline]
  pub fn build<T: 'static>(
    self,
    _window_target: &EventLoopWindowTarget<T>,
  ) -> Result<platform_impl::SystemTray, OsError> {
    self.0.build(&_window_target)
  }
}

#[cfg(not(feature = "tray"))]
impl SystemTrayBuilder {
  #[inline]
  #[cfg(not(target_os = "linux"))]
  pub fn new(_icon: Vec<u8>, _tray_menu: Option<ContextMenu>) -> Self {
    Self
  }
  #[inline]
  #[cfg(target_os = "linux")]
  pub fn new(_icon: PathBuf, _tray_menu: Option<ContextMenu>) -> Self {
    Self
  }
  pub fn build<T: 'static>(
    self,
    _window_target: &EventLoopWindowTarget<T>,
  ) -> Result<SystemTray, String> {
    Err("tray not supported on this platform".into())
  }
}

#[cfg(not(feature = "tray"))]
impl SystemTray {
  #[inline]
  #[cfg(not(target_os = "linux"))]
  pub fn set_icon(&mut self, _icon: Vec<u8>) {}
  #[inline]
  #[cfg(target_os = "linux")]
  pub fn set_icon(&mut self, _icon: PathBuf) {}
}
