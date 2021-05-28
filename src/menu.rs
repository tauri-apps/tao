// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};

use crate::platform_impl::{CustomMenuItem, Menu as MenuPlatform};

// test with shortcuts, we can remove them as well?
pub struct Tray;
pub struct Menubar;

impl Tray {
  pub fn new() -> Menu {
    Menu {
      menu_platform: MenuPlatform::new_popup_menu(),
      menu_type: MenuType::SystemTray,
    }
  }
}

impl Menubar {
  pub fn new() -> Menu {
    Menu {
      menu_platform: MenuPlatform::new(),
      menu_type: MenuType::Menubar,
    }
  }
}

// menu builder
#[derive(Debug, Clone)]
pub struct Menu {
  pub(crate) menu_platform: MenuPlatform,
  pub(crate) menu_type: MenuType,
}

impl Menu {
  pub fn new(menu_type: MenuType) -> Self {
    Self {
      menu_platform: MenuPlatform::new(),
      menu_type,
    }
  }

  pub fn new_popup_menu(menu_type: MenuType) -> Self {
    Self {
      menu_platform: MenuPlatform::new_popup_menu(),
      menu_type,
    }
  }

  pub fn add_children(&mut self, children: Menu, title: &str, enabled: bool) {
    self
      .menu_platform
      .add_children(children.menu_platform, title, enabled);
  }

  pub fn add_custom_item(
    &mut self,
    text: &str,
    keyboard_accelerator: Option<&str>,
    enabled: bool,
    selected: bool,
  ) -> (MenuId, CustomMenuItem) {
    let menu_id = MenuId::new(&text);
    let item = self.menu_platform.add_custom_item(
      menu_id,
      self.menu_type,
      text,
      keyboard_accelerator,
      enabled,
      selected,
    );
    (menu_id, item)
  }

  pub fn add_item(&mut self, item: MenuAction) -> Option<CustomMenuItem> {
    self.menu_platform.add_system_item(item, self.menu_type)
  }

  pub fn add_separator(&mut self) {
    self.menu_platform.add_separator()
  }
}

/// A menu item, bound to a pre-defined action or `Custom` emit an event. Note that status bar only
/// supports `Custom` menu item variants. And on the menu bar, some platforms might not support some
/// of the variants. Unsupported variant will be no-op on such platform.
#[derive(Debug, Clone)]
pub enum MenuAction {
  Custom(CustomMenuItem),
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
  /// - **Windows / Android / iOS:** Unsupported
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
  /// - **Windows / Android / iOS:** Unsupported
  ///
  CloseWindow,

  /// A "quit this app" menu icon.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Android / iOS:** Unsupported
  ///
  Quit,

  /// A menu item for enabling copying (often text) from responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Android / iOS:** Unsupported
  /// - **Linux**: require `menu` feature flag
  ///
  Copy,

  /// A menu item for enabling cutting (often text) from responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Android / iOS:** Unsupported
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
  /// - **Windows / Android / iOS:** Unsupported
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
  /// - **Windows / Android / iOS:** Unsupported
  ///
  Minimize,

  /// An item for instructing the app to zoom
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  Zoom,

  // FIXME: Add description,
  Separator,
  Children(String, MenuPlatform),
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
  fn new(menu_title: &str) -> MenuId {
    MenuId(hash_string_to_u32(menu_title))
  }
}

/// Type of menu the click is originating from.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MenuType {
  /// Menubar menu item.
  Menubar,
  /// System tray menu item.
  SystemTray,
}

fn hash_string_to_u32(title: &str) -> u32 {
  let mut s = DefaultHasher::new();
  title.hash(&mut s);
  s.finish() as u32
}

pub enum MenuIcon {
  // green dot
  StatusAvailable,
  // yellow dot
  StatusPartiallyAvailable,
  // red dot
  StatusUnavailable,
}
