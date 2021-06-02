// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};

use crate::platform_impl::{CustomMenuItem as CustomMenuItemPlatform, Menu as MenuPlatform};

/// Object that allows you to build a `ContextMenu`, for the tray.
pub struct ContextMenu(pub(crate) Menu);
/// Object that allows you to build a `MenuBar`, for the *Window* menu in Windows and Linux and the *Menu bar* on macOS.
pub struct MenuBar(pub(crate) Menu);

/// A custom menu item.
pub struct CustomMenuItem<'a> {
  id: MenuId,
  title: &'a str,
  keyboard_accelerator: Option<&'a str>,
  enabled: bool,
  selected: bool,
}

impl<'a> CustomMenuItem<'a> {
  /// Creates a new custom menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Linux:** `selected` render a regular item
  pub fn new(
    title: &'a str,
    keyboard_accelerator: Option<&'a str>,
    enabled: bool,
    selected: bool,
  ) -> Self {
    Self {
      id: MenuId::new(title),
      title,
      keyboard_accelerator,
      enabled,
      selected,
    }
  }

  /// Sets the menu id.
  pub fn with_id(mut self, id: MenuId) -> Self {
    self.id = id;
    self
  }
}

/// Base `Menu` functions.
///
/// See `ContextMenu` or `MenuBar` to build your menu.
#[derive(Debug, Clone)]
pub(crate) struct Menu {
  pub(crate) menu_platform: MenuPlatform,
  pub(crate) menu_type: MenuType,
}

impl ContextMenu {
  /// Creates a new Menu for context (popup, tray etc..).
  pub fn new() -> Self {
    Self(Menu {
      menu_platform: MenuPlatform::new_popup_menu(),
      menu_type: MenuType::ContextMenu,
    })
  }

  /// Add a submenu.
  pub fn add_submenu(&mut self, title: &str, enabled: bool, submenu: ContextMenu) {
    self
      .0
      .menu_platform
      .add_submenu(title, enabled, submenu.0.menu_platform);
  }

  /// Add new item to this menu.
  pub fn add_item(&mut self, item: CustomMenuItem<'_>) -> CustomMenuItemHandle {
    self.0.menu_platform.add_item(
      item.id,
      item.title,
      item.keyboard_accelerator,
      item.enabled,
      item.selected,
      MenuType::ContextMenu,
    )
  }

  /// Add new item to this menu.
  pub fn add_native_item(&mut self, item: MenuItem) -> Option<CustomMenuItemHandle> {
    self.0.menu_platform.add_native_item(item, self.0.menu_type)
  }
}

impl Default for ContextMenu {
  fn default() -> Self {
    Self::new()
  }
}

impl MenuBar {
  /// Creates a new Menubar (Window) menu for platforms where this is appropriate.
  pub fn new() -> Self {
    Self(Menu {
      menu_platform: MenuPlatform::new(),
      menu_type: MenuType::MenuBar,
    })
  }

  /// Add a submenu.
  pub fn add_submenu(&mut self, title: &str, enabled: bool, submenu: MenuBar) {
    self
      .0
      .menu_platform
      .add_submenu(title, enabled, submenu.0.menu_platform);
  }

  /// Add new item to this menu.
  pub fn add_item(&mut self, item: CustomMenuItem<'_>) -> CustomMenuItemHandle {
    self.0.menu_platform.add_item(
      item.id,
      item.title,
      item.keyboard_accelerator,
      item.enabled,
      item.selected,
      MenuType::MenuBar,
    )
  }

  /// Add new item to this menu.
  pub fn add_native_item(&mut self, item: MenuItem) -> Option<CustomMenuItemHandle> {
    self.0.menu_platform.add_native_item(item, self.0.menu_type)
  }
}

impl Default for MenuBar {
  fn default() -> Self {
    Self::new()
  }
}

/// A menu item, bound to a pre-defined native action. Note some platforms might not
/// support some of the variants. Unsupported variant will be no-op on such platform.
#[derive(Debug, Clone)]
pub enum MenuItem {
  /// Shows a standard "About" item
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Android / iOS:** Unsupported
  ///
  About(String),

  /// A standard "hide the app" menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  Hide,

  /// A standard "Services" menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  Services,

  /// A "hide all other windows" menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  HideOthers,

  /// A menu item to show all the windows for this app.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  ShowAll,

  /// Close the current window.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  CloseWindow,

  /// A "quit this app" menu icon.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  Quit,

  /// A menu item for enabling copying (often text) from responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  /// - **Linux**: require `menu` feature flag
  ///
  Copy,

  /// A menu item for enabling cutting (often text) from responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  /// - **Linux**: require `menu` feature flag
  ///
  Cut,

  /// An "undo" menu item; particularly useful for supporting the cut/copy/paste/undo lifecycle
  /// of events.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  Undo,

  /// An "redo" menu item; particularly useful for supporting the cut/copy/paste/undo lifecycle
  /// of events.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  Redo,

  /// A menu item for selecting all (often text) from responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Android / iOS:** Unsupported
  /// - **Linux**: require `menu` feature flag
  ///
  SelectAll,

  /// A menu item for pasting (often text) into responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  /// - **Linux**: require `menu` feature flag
  ///
  Paste,

  /// A standard "enter full screen" item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  EnterFullScreen,

  /// An item for minimizing the window with the standard system controls.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  Minimize,

  /// An item for instructing the app to zoom
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  Zoom,

  /// Represents a Separator
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  Separator,
}

/// Custom menu item, when clicked an event is emitted in the EventLoop.
/// You can modify the item after it's creation.
#[derive(Debug, Clone)]
pub struct CustomMenuItemHandle(pub CustomMenuItemPlatform);

/// Base `CustomMenuItemHandle` functions.
impl CustomMenuItemHandle {
  /// Returns an identifier unique to the menu item.
  pub fn id(self) -> MenuId {
    self.0.id()
  }

  /// Modifies the status of the menu item.
  pub fn set_enabled(&mut self, is_enabled: bool) {
    self.0.set_enabled(is_enabled)
  }

  /// Modifies the title (label) of the menu item.
  pub fn set_title(&mut self, title: &str) {
    self.0.set_title(title)
  }

  /// Modifies the selected state of the menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Linux:** Unsupported, render a regular item
  pub fn set_selected(&mut self, is_selected: bool) {
    self.0.set_selected(is_selected)
  }

  // todo: Add set_icon
  // pub fn set_icon(&mut self, icon: Vec<u8>) {
  //   self.0.set_icon(icon)
  // }
}

/// Identifier of a custom menu item.
///
/// Whenever you receive an event arising from a particular menu, this event contains a `MenuId` which
/// identifies its origin.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct MenuId(pub u32);

impl From<MenuId> for u32 {
  fn from(s: MenuId) -> u32 {
    s.0
  }
}

impl MenuId {
  /// Return an empty `MenuId`.
  pub const EMPTY: MenuId = MenuId(0);

  /// Create new `MenuId` from a String.
  pub fn new(unique_string: &str) -> MenuId {
    MenuId(hash_string_to_u32(unique_string))
  }

  /// Whenever this menu is empty.
  pub fn is_empty(self) -> bool {
    Self::EMPTY == self
  }
}

/// Type of menu the click is originating from.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MenuType {
  /// Menubar menu item.
  MenuBar,
  /// System tray menu item.
  ContextMenu,
}

fn hash_string_to_u32(title: &str) -> u32 {
  let mut s = DefaultHasher::new();
  title.hash(&mut s);
  s.finish() as u32
}
