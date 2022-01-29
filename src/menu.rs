// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

//! **UNSTABLE** -- The [`Menu`] struct and associated types.
//!
//! ```rust,ignore
//! let mut menu_bar = Menu::new();
//! let mut file_menu = Menu::with_title("File");
//! menu_bar.add_submenu(&file_menu);
//! let mut item = CustomMenuItem::new("New Menu Item", true, false, None).unwrap();
//! file_menu.add_custom_item(&item);
//! item.set_enabled(false);
//! ```

use std::{collections::HashMap, sync::Mutex};

use crate::{accelerator::Accelerator, error::OsError, platform_impl};

pub type MenuId = u16;

/// Represets a Menu that can be used as a menu bar, a context menu, or a submenu.
///
/// ## Platform-specific
///
/// - **Android / iOS:** Unsupported.
/// - **Windows / Linux:** If used as a menu bar, it will apear at the top of the window.
/// - **macOs:** if used as a menu bar, it should be used with [`crate::event_loop::EventLoop`] and it will apear in the macOS menu bar.
#[derive(Debug, Clone)]
pub struct Menu(MenuId);
impl Menu {
  /// Creates a new [`Menu`] without a title, suitable to be used as a menu bar, or a context menu.
  pub fn new() -> Result<Self, OsError> {
    Self::with_title("")
  }

  /// Creates a new [`Menu`] with a title, suitable to be used as a submenu inside a menu bar, or a contex menu, or inside another submenu.
  pub fn with_title(title: &str) -> Result<Self, OsError> {
    platform_impl::Menu::new(title).map(|i| Self(i))
  }

  /// Returns a unique identifier to the menu.
  pub fn id(&self) -> MenuId {
    self.0
  }

  /// Adds a custom item to this menu.
  pub fn add_custom_item(&mut self, item: &CustomMenuItem) {
    platform_impl::Menu::add_custom_item(self.id(), item.id())
  }

  /// Adds a native item to this menu.
  pub fn add_native_item(&mut self, item: NativeMenuItem) {
    platform_impl::Menu::add_native_item(self.id(), item)
  }

  /// Adds a submenu to this menu.
  pub fn add_submenu(&mut self, menu: &Menu) {
    platform_impl::Menu::add_submenu(self.id(), menu.id())
  }
}

/// Represets a custom menu item that can be used in [`Menu`]s.
#[derive(Debug, Clone)]
pub struct CustomMenuItem(MenuId);
impl CustomMenuItem {
  /// Creates a new [`CustomMenuItem`]
  pub fn new(
    title: &str,
    enabled: bool,
    selected: bool,
    accel: Option<Accelerator>,
  ) -> Result<Self, OsError> {
    platform_impl::CustomMenuItem::new(title, enabled, selected, accel).map(|i| Self(i))
  }

  /// Returns a unique identifier to the menu item.
  pub fn id(&self) -> MenuId {
    self.0
  }

  /// Modifies the title (label) of the menu item.
  pub fn set_title(&mut self, title: &str) {
    platform_impl::CustomMenuItem::set_title(self.id(), title)
  }

  /// Enables or disables the menu item.
  pub fn set_enabled(&mut self, enabled: bool) {
    platform_impl::CustomMenuItem::set_enabled(self.id(), enabled)
  }

  /// Modifies the selected state of the menu item.
  pub fn set_selected(&mut self, selected: bool) {
    platform_impl::CustomMenuItem::set_selected(self.id(), selected)
  }
}

/// A menu item, bound to a pre-defined native action.
///
/// ## Platform-specific
///
/// - **Android / iOS:** Unsupported
///
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum NativeMenuItem {
  /// A native Separator menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  Separator,
  /// A native item to copy selected (often text).
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  Copy,
  /// A native item to cut selected (often text).
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  Cut,
  /// A native item to paste (often text).
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  Paste,
  /// A native item to select all.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  SelectAll,
  /// A native item to minimize the window.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  Minimize,
  /// A native item to hide the window.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  Hide,
  /// A native item to close the window.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  CloseWindow,
  /// A native item to quit the app.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  Quit,
  /// A native macOS "Services" menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  Services,
  /// A native item to hide all other windows.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  HideOthers,
  /// A native item to show all windows.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  ShowAll,
  /// A native item to undo actions; particularly useful for supporting the cut/copy/paste/undo/redo lifecycle.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  Undo,
  /// A native item to redo actions; particularly useful for supporting the cut/copy/paste/undo/redo lifecycle.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  Redo,
  /// A native item to enter full screen window.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Unsupported
  ///
  EnterFullScreen,
  /// A native item to instruct the window to zoom
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported
  ///
  Zoom,
}

impl NativeMenuItem {
  pub(crate) fn id(&self) -> MenuId {
    match self {
      NativeMenuItem::Separator => 6556,
      NativeMenuItem::Copy => 6547,
      NativeMenuItem::Cut => 6548,
      NativeMenuItem::Paste => 6552,
      NativeMenuItem::SelectAll => 6551,
      NativeMenuItem::Minimize => 6554,
      NativeMenuItem::Hide => 6541,
      NativeMenuItem::CloseWindow => 6545,
      NativeMenuItem::Quit => 6546,
      NativeMenuItem::Services => 6542,
      NativeMenuItem::HideOthers => 6543,
      NativeMenuItem::ShowAll => 6544,
      NativeMenuItem::Undo => 6549,
      NativeMenuItem::Redo => 6550,
      NativeMenuItem::EnterFullScreen => 6553,
      NativeMenuItem::Zoom => 6555,
    }
  }

  #[must_use]
  pub(crate) fn from_id(id: MenuId) -> Self {
    match id {
      6556 => NativeMenuItem::Separator,
      6547 => NativeMenuItem::Copy,
      6548 => NativeMenuItem::Cut,
      6552 => NativeMenuItem::Paste,
      6551 => NativeMenuItem::SelectAll,
      6554 => NativeMenuItem::Minimize,
      6541 => NativeMenuItem::Hide,
      6545 => NativeMenuItem::CloseWindow,
      6546 => NativeMenuItem::Quit,
      6542 => NativeMenuItem::Services,
      6543 => NativeMenuItem::HideOthers,
      6544 => NativeMenuItem::ShowAll,
      6549 => NativeMenuItem::Undo,
      6550 => NativeMenuItem::Redo,
      6553 => NativeMenuItem::EnterFullScreen,
      6555 => NativeMenuItem::Zoom,
      _ => unreachable!(),
    }
  }
}

/// A struct to hold all menus and menu items data internall.
///
/// - A [`platform_impl::Menu`] holds IDs of its children, which can be either a [`NativeMenuItem`], [`platform_impl::CustomMenuItem`], or another [`platform_impl::Menu`]
/// - A [`platform_impl::CustomMenuItem`] can be added to multiple [`platform_impl::Menu`]s at the same time,
///   so it holds IDs of its parent menus. This makes it easier for us to update all
///   instances of the custom item if needed.
pub(crate) struct MenusData {
  pub(crate) menus: HashMap<MenuId, platform_impl::Menu>,
  pub(crate) custom_menu_items: HashMap<MenuId, platform_impl::CustomMenuItem>,
}

lazy_static! {
  pub(crate) static ref MENUS_DATA: Mutex<MenusData> = Mutex::new(MenusData {
    menus: HashMap::new(),
    custom_menu_items: HashMap::new()
  });
}

#[derive(Clone, Copy)]
pub(crate) enum MenuItemType {
  Custom,
  Submenu,
  NativeItem,
}
