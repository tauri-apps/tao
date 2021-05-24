// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};
use crate::keyboard::Hotkey;


#[derive(Debug, Clone)]
pub struct Menu {
  pub title: String,
  pub items: Vec<MenuItem>,
}

impl Menu {
  pub fn new(title: &str, items: Vec<MenuItem>) -> Self {
    Self {
      title: String::from(title),
      items,
    }
  }
}

#[derive(Debug, Clone, Hash)]
/// CustomMenu is a custom menu who emit an event inside the EventLoop.
pub struct CustomMenu {
  pub id: MenuId,
  pub name: String,
  pub keyboard_accelerators: Option<Hotkey>,
}

/// A menu item, bound to a pre-defined action or `Custom` emit an event. Note that status bar only
/// supports `Custom` menu item variants. And on the menu bar, some platforms might not support some
/// of the variants. Unsupported variant will be no-op on such platform.
#[derive(Debug, Clone)]
pub enum MenuItem {
  /// A custom menu emit an event inside the EventLoop.
  Custom(CustomMenu),

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

  /// Represents a Separator
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Android / iOS:** Unsupported
  ///
  Separator,
}

impl MenuItem {
  /// Create new custom menu item.
  /// unique_menu_id is the unique ID for the menu item returned in the EventLoop `Event::MenuEvent(unique_menu_id)`
  pub fn new(title: impl ToString) -> Self {
    let title = title.to_string();
    MenuItem::Custom(CustomMenu {
      id: MenuId::new(&title),
      name: title,
      keyboard_accelerators: None,
    })
  }

  /// Assign keyboard shortcut to the menu action. Works only with `MenuItem::Custom`.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Android / iOS:** Unsupported
  ///
  pub fn with_accelerators(mut self, keyboard_accelerators: Hotkey) -> Self {
    if let MenuItem::Custom(ref mut custom_menu) = self {
      custom_menu.keyboard_accelerators = Some(keyboard_accelerators);
    }
    self
  }

  /// Return unique menu ID. Works only with `MenuItem::Custom`.
  pub fn id(&self) -> MenuId {
    if let MenuItem::Custom(custom_menu) = self {
      return custom_menu.id;
    }

    // return blank menu id if we request under a non-custom menu
    // this prevent to wrap it inside an Option<>
    MenuId(4294967295)
  }
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
