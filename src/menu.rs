// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};

use crate::platform_impl::{CustomMenuItem, Menu as MenuPlatform};

// Tray menu (Also known as context menu)
pub struct Tray;
// Menubar menu (It represent the Window menu for Windows and Linux)
pub struct Menubar;

impl Tray {
  /// Creates a new Tray (Context) menu for platforms where this is appropriate.
  ///
  /// This function is equivalent to [`Menu::new_popup_menu(MenuType::SystemTray)`].
  /// [`Menu::new_popup_menu(MenuType::SystemTray)`]: crate::menu::Menu
  #[allow(clippy::new_ret_no_self)]
  pub fn new() -> Menu {
    Menu {
      menu_platform: MenuPlatform::new_popup_menu(),
      menu_type: MenuType::SystemTray,
    }
  }
}

impl Menubar {
  /// Creates a new Menubar (Window) menu for platforms where this is appropriate.
  ///
  /// This function is equivalent to [`Menu::new(MenuType::Menubar)`].
  /// [`Menu::new_popup_menu(MenuType::Menubar)`]: crate::menu::Menu
  #[allow(clippy::new_ret_no_self)]
  pub fn new() -> Menu {
    Menu {
      menu_platform: MenuPlatform::new(),
      menu_type: MenuType::Menubar,
    }
  }
}

/// Base `Menu` functions.
#[derive(Debug, Clone)]
pub struct Menu {
  pub(crate) menu_platform: MenuPlatform,
  pub(crate) menu_type: MenuType,
}

impl Menu {
  /// Creates a new Menu for Menubar/Window context.
  pub fn new(menu_type: MenuType) -> Self {
    Self {
      menu_platform: MenuPlatform::new(),
      menu_type,
    }
  }

  /// Creates a new Menu for Popup context.
  pub fn new_popup_menu(menu_type: MenuType) -> Self {
    Self {
      menu_platform: MenuPlatform::new_popup_menu(),
      menu_type,
    }
  }

  /// Shortcut to add a submenu to this `Menu`.
  ///
  /// This function is equivalent to [`Menu::add_item(self, MenuItem::Submenu(title, enabled, submenu))`].
  /// See [`Menu::add_item`] for details.
  ///
  /// [`Menu::add_item`]: crate::menu::Menu
  pub fn add_submenu(&mut self, title: &str, enabled: bool, submenu: Menu) {
    self.menu_platform.add_item(
      MenuItem::Submenu(title.to_string(), enabled, submenu.menu_platform),
      self.menu_type,
    );
  }

  /// Add new item to this menu.
  pub fn add_item(&mut self, item: MenuItem) -> Option<CustomMenuItem> {
    self.menu_platform.add_item(item, self.menu_type)
  }
}

/// A menu item, bound to a pre-defined action or `Custom` emit an event. Note some platforms
/// might not support some of the variants. Unsupported variant will be no-op on such platform.
/// Use `MenuItem::new` to create a `CustomMenuItem`.
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
    platform_item: Option<CustomMenuItem>,
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
  Submenu(String, bool, MenuPlatform),
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
      platform_item: None,
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
  fn new(menu_title: &str) -> MenuId {
    MenuId(hash_string_to_u32(menu_title))
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
  Menubar,
  /// System tray menu item.
  SystemTray,
}

fn hash_string_to_u32(title: &str) -> u32 {
  let mut s = DefaultHasher::new();
  title.hash(&mut s);
  s.finish() as u32
}
