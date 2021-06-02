// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
  ops::Deref,
};

use crate::platform_impl::{CustomMenuItem as CustomMenuItemPlatform, Menu as MenuPlatform};

/// Context menu to be used in tray only for now.
pub struct ContextMenu(pub(crate) Menu);
/// Menubar menu represent the Window menu for Windows and Linux and the App menu for macOS).
pub struct MenuBar(pub(crate) Menu);

/// Base `Menu` functions.
///
/// See `ContextMenu` or `MenuBar` to build your menu.
#[derive(Debug, Clone)]
pub struct Menu {
  pub(crate) menu_platform: MenuPlatform,
  pub(crate) menu_type: MenuType,
}

impl Deref for ContextMenu {
  type Target = Menu;
  fn deref(&self) -> &Menu {
    &self.0
  }
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
    self.0.menu_platform.add_item(
      MenuItem::Submenu {
        enabled,
        menu_platform: submenu.0.menu_platform,
        title: title.to_string(),
      },
      self.menu_type,
    );
  }

  /// Add new item to this menu.
  pub fn add_item(&mut self, item: MenuItem) -> Option<CustomMenuItem> {
    self.0.menu_platform.add_item(item, self.menu_type)
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
    self.0.menu_platform.add_item(
      MenuItem::Submenu {
        enabled,
        menu_platform: submenu.0.menu_platform,
        title: title.to_string(),
      },
      self.0.menu_type,
    );
  }

  /// Add new item to this menu.
  pub fn add_item(&mut self, item: MenuItem) -> Option<CustomMenuItem> {
    self.0.menu_platform.add_item(item, self.0.menu_type)
  }
}

impl Default for MenuBar {
  fn default() -> Self {
    Self::new()
  }
}

/// A menu item, bound to a pre-defined action or `Custom` emit an event. Note some platforms
/// might not support some of the variants. Unsupported variant will be no-op on such platform.
/// Tip: Use `MenuItem::new` shortcut to create a `CustomMenuItem`.
#[derive(Debug, Clone)]
pub enum MenuItem {
  /// A custom menu emit an event inside the EventLoop.
  /// Use `Menu::add_custom_item` to create a new custom item.
  Custom {
    menu_id: MenuId,
    text: String,
    keyboard_accelerator: Option<String>,
    enabled: bool,
    selected: bool,
  },

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

  /// Represents a Submenu(title, enabled, sub menu)
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  Submenu {
    title: String,
    enabled: bool,
    menu_platform: MenuPlatform,
  },
}

/// Base `MenuItem` functions.
impl MenuItem {
  /// Creates a new `MenuItem::Custom`.
  pub fn new(menu_title: &str) -> Self {
    let title = menu_title.to_string();
    MenuItem::Custom {
      menu_id: MenuId::new(&title),
      text: title,
      keyboard_accelerator: None,
      enabled: true,
      selected: false,
    }
  }

  /// Sets whether the menu will be initially with keyboards accelerators or not.
  pub fn with_accelerators(mut self, keyboard_accelerators: impl ToString) -> Self {
    if let MenuItem::Custom {
      ref mut keyboard_accelerator,
      ..
    } = self
    {
      *keyboard_accelerator = Some(keyboard_accelerators.to_string());
    }
    self
  }

  /// Sets whether the menu will be initially enabled or not.
  pub fn with_enabled(mut self, is_enabled: bool) -> Self {
    if let MenuItem::Custom {
      ref mut enabled, ..
    } = self
    {
      *enabled = is_enabled;
    }
    self
  }

  /// Sets whether the menu will be initially selected or not.
  pub fn with_selected(mut self, is_selected: bool) -> Self {
    if let MenuItem::Custom {
      ref mut selected, ..
    } = self
    {
      *selected = is_selected;
    }
    self
  }
}

/// Custom menu item, when clicked an event is emitted in the EventLoop.
/// You can modify the item after it's creation.
#[derive(Debug, Clone)]
pub struct CustomMenuItem(pub CustomMenuItemPlatform);

/// Base `CustomMenuItem` functions.
impl CustomMenuItem {
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
