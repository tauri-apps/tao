// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

//! **UNSTABLE** -- The `Menu` struct and associated types.
//!
//! [ContextMenu][context_menu] is used to created a tray menu.
//!
//! [MenuBar][menu_bar] is used to created a Window menu on Windows and Linux. On macOS it's used in the menubar.
//!
//! ```rust,ignore
//! let mut root_menu = MenuBar::new();
//! let mut file_menu = MenuBar::new();
//!
//! file_menu.add_item(MenuItemAttributes::new("My menu item"));
//! root_menu.add_submenu("File", true, file_menu);
//! ```
//!
//! [menu_bar]: crate::menu::MenuBar
//! [context_menu]: crate::menu::ContextMenu

use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
  path::PathBuf,
};

use crate::{
  accelerator::Accelerator,
  platform_impl::{Menu as MenuPlatform, MenuItemAttributes as CustomMenuItemPlatform},
  window::Icon,
};

/// Object that allows you to create a `ContextMenu`.
///
/// ## Platform-specific
///
pub struct ContextMenu(pub(crate) Menu);
/// Object that allows you to create a `MenuBar`, menu.
///
/// ## Platform-specific
///
/// **macOs:** The menu will show in the **Menu Bar**.
/// **Linux / Windows:** The menu will be show at the top of the window.
pub struct MenuBar(pub(crate) Menu);

/// A custom menu item.
pub struct MenuItemAttributes<'a> {
  id: MenuId,
  title: &'a str,
  keyboard_accelerator: Option<Accelerator>,
  enabled: bool,
  selected: bool,
}

impl<'a> MenuItemAttributes<'a> {
  /// Creates a new custom menu item.
  ///
  /// `title` could optionally contain an `&` before a character to assign this character as the mnemonic
  /// for this menu item. To display a `&` without assigning a mnemenonic, use `&&`
  ///
  /// ## Platform-specific:
  ///
  /// - **macOS**: mnemonics are not supported but single `&` will be rmeoved anyways for consistency with other platforms.
  pub fn new(title: &'a str) -> Self {
    Self {
      id: MenuId::new(title),
      title,
      keyboard_accelerator: None,
      enabled: true,
      selected: false,
    }
  }

  /// Assign a custom menu id.
  pub fn with_id(mut self, id: MenuId) -> Self {
    self.id = id;
    self
  }

  /// Assign keyboard shortcut to the menu action.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Android / iOS:** Unsupported
  ///
  pub fn with_accelerators(mut self, keyboard_accelerators: &Accelerator) -> Self {
    self.keyboard_accelerator = Some(keyboard_accelerators.to_owned());
    self
  }

  /// Assign default menu state.
  pub fn with_enabled(mut self, enabled: bool) -> Self {
    self.enabled = enabled;
    self
  }

  /// Assign default checkbox style.
  ///
  /// Default is `false`
  ///
  /// If `selected` is `false`, renders a regular menu item.
  pub fn with_selected(mut self, selected: bool) -> Self {
    self.selected = selected;
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
  ///
  /// `title` could optionally contain an `&` before a character to assign this character as the mnemonic
  /// for this menu item. To display a `&` without assigning a mnemenonic, use `&&`
  ///
  /// ## Platform-specific:
  ///
  /// - **macOS**: mnemonics are not supported but single `&` will be rmeoved anyways for consistency with other platforms.
  pub fn add_submenu(&mut self, title: &str, enabled: bool, submenu: ContextMenu) {
    self
      .0
      .menu_platform
      .add_submenu(title, enabled, submenu.0.menu_platform);
  }

  /// Add new item to this menu.
  pub fn add_item(&mut self, item: MenuItemAttributes<'_>) -> CustomMenuItem {
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
  pub fn add_native_item(&mut self, item: MenuItem) -> Option<CustomMenuItem> {
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
  ///
  /// `title` could optionally contain an `&` before a character to assign this character as the mnemonic
  /// for this menu item. To display a `&` without assigning a mnemenonic, use `&&`
  ///
  /// ## Platform-specific:
  ///
  /// - **macOS**: mnemonics are not supported but single `&` will be rmeoved anyways for consistency with other platforms.
  pub fn add_submenu(&mut self, title: &str, enabled: bool, submenu: MenuBar) {
    self
      .0
      .menu_platform
      .add_submenu(title, enabled, submenu.0.menu_platform);
  }

  /// Add new item to this menu.
  pub fn add_item(&mut self, item: MenuItemAttributes<'_>) -> CustomMenuItem {
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
  pub fn add_native_item(&mut self, item: MenuItem) -> Option<CustomMenuItem> {
    self.0.menu_platform.add_native_item(item, self.0.menu_type)
  }
}

impl Default for MenuBar {
  fn default() -> Self {
    Self::new()
  }
}

/// Application metadata for the [`MenuItem::About`] action.
///
/// ## Platform-specific
///
/// - **Windows / macOS / Android / iOS:** The metadata is ignored on these platforms.
#[derive(Debug, Clone, Default)]
pub struct AboutMetadata {
  /// The application name.
  pub version: Option<String>,
  /// The authors of the application.
  pub authors: Option<Vec<String>>,
  /// Application comments.
  pub comments: Option<String>,
  /// The copyright of the application.
  pub copyright: Option<String>,
  /// The license of the application.
  pub license: Option<String>,
  /// The application website.
  pub website: Option<String>,
  /// The website label.
  pub website_label: Option<String>,
}

/// Contents to share to other services.
#[derive(Debug, Clone)]
pub struct SharingItem {
  pub texts: Option<Vec<String>>,
  pub urls: Option<Vec<String>>,
  pub file_paths: Option<Vec<PathBuf>>,
}

/// A menu item, bound to a pre-defined native action.
///
/// Note some platforms might not support some of the variants.
/// Unsupported variant will be no-op on such platform.
///
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum MenuItem {
  /// Shows a standard "About" item.
  ///
  /// The first value is the application name, and the second is its metadata.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Android / iOS:** Unsupported
  /// - **Linux:** The metadata is only applied on Linux
  ///
  About(String, AboutMetadata),

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
  /// - **Android / iOS / Linux:** Unsupported
  ///
  Copy,

  /// A menu item for enabling cutting (often text) from responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS / Linux:** Unsupported
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
  /// - **Windows / Android / iOS / Linux:** Unsupported
  ///
  SelectAll,

  /// A menu item for pasting (often text) into responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS / Linux:** Unsupported
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

  /// A menu item for listing available services to share contents.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  Share(SharingItem),
}

/// Custom menu item, when clicked an event is emitted in the EventLoop.
///
/// You can modify the item after it's creation.
#[derive(Debug, Clone)]
pub struct CustomMenuItem(pub CustomMenuItemPlatform);

/// Base `CustomMenuItem` functions.
impl CustomMenuItem {
  /// Returns an identifier unique to the menu item.
  pub fn id(self) -> MenuId {
    self.0.id()
  }

  /// Returns the title(label) of the menu item.
  pub fn title(&self) -> String {
    self.0.title()
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

  /// Set icon of the menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux:** Not implemented
  pub fn set_icon(&mut self, icon: Icon) {
    self.0.set_icon(icon)
  }
}

/// Identifier of a custom menu item.
///
/// Whenever you receive an event arising from a particular menu, this event contains a `MenuId` which
/// identifies its origin.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct MenuId(pub u16);

impl From<MenuId> for u16 {
  fn from(s: MenuId) -> u16 {
    s.0
  }
}

impl MenuId {
  /// Return an empty `MenuId`.
  pub const EMPTY: MenuId = MenuId(0);

  /// Create new `MenuId` from a String.
  pub fn new(unique_string: &str) -> MenuId {
    MenuId(hash_string_to_u16(unique_string))
  }

  /// Whenever this menu is empty.
  pub fn is_empty(self) -> bool {
    Self::EMPTY == self
  }
}

/// Type of menu the click is originating from.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MenuType {
  /// Menubar menu item.
  MenuBar,
  /// System tray menu item.
  ContextMenu,
}

fn hash_string_to_u16(title: &str) -> u16 {
  let mut s = DefaultHasher::new();
  title.to_uppercase().hash(&mut s);
  s.finish() as u16
}
