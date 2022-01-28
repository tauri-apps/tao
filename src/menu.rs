// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{
  accelerator::Accelerator,
  error::{ExternalError, OsError},
  platform_impl,
};

pub type MenuId = u16;

#[derive(Debug, Clone, Copy)]
pub struct Menu(MenuId);
impl Menu {
  pub fn new() -> Result<Self, OsError> {
    Self::with_title("")
  }

  pub fn with_title(title: &str) -> Result<Self, OsError> {
    platform_impl::Menu::new(title).map(|i| Self(i))
  }

  pub fn id(&self) -> MenuId {
    self.0
  }

  pub fn add_submenu(&self, menu: &Menu) {
    platform_impl::Menu::add_submenu(self.id(), menu.id())
  }

  pub fn add_custom_item(&self, item: &CustomMenuItem) {
    platform_impl::Menu::add_custom_item(self.id(), item.id())
  }
}

#[derive(Debug, Clone)]
pub struct CustomMenuItem(MenuId);
impl CustomMenuItem {
  pub fn new(
    title: &str,
    enabled: bool,
    selected: bool,
    accel: Option<Accelerator>,
  ) -> Result<Self, OsError> {
    platform_impl::CustomMenuItem::new(title, enabled, selected, accel).map(|i| Self(i))
  }

  pub fn id(&self) -> MenuId {
    self.0
  }
}

/// A menu item, bound to a pre-defined native action.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum NativeMenuItem {
  About(String),
  Hide,
  Services,
  HideOthers,
  ShowAll,
  CloseWindow,
  Quit,
  Copy,
  Cut,
  Undo,
  Redo,
  SelectAll,
  Paste,
  EnterFullScreen,
  Minimize,
  Zoom,
  Separator,
}
