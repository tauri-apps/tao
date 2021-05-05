use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};

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

#[derive(Debug, Clone, Hash, Copy)]
/// CustomMenu is a custom menu who emit an event inside the EventLoop.
pub struct CustomMenu {
  pub _id: MenuId,
  pub name: &'static str,
  pub keyboard_accelerators: Option<&'static str>,
}

/// A menu item, binded to a pre-defined action or `Custom` emit an event.
#[derive(Debug, Clone, Copy)]
pub enum MenuItem {
  /// A custom menu emit an event inside the EventLoop.
  Custom(CustomMenu),

  /// Shows a standard "About" item
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  About(&'static str),

  /// A standard "hide the app" menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  Hide,

  /// A standard "Services" menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  Services,

  /// A "hide all other windows" menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  HideOthers,

  /// A menu item to show all the windows for this app.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  ShowAll,

  /// Close the current window.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  CloseWindow,

  /// A "quit this app" menu icon.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  Quit,

  /// A menu item for enabling copying (often text) from responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  Copy,

  /// A menu item for enabling cutting (often text) from responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  Cut,

  /// An "undo" menu item; particularly useful for supporting the cut/copy/paste/undo lifecycle
  /// of events.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  Undo,

  /// An "redo" menu item; particularly useful for supporting the cut/copy/paste/undo lifecycle
  /// of events.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  Redo,

  /// A menu item for selecting all (often text) from responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  SelectAll,

  /// A menu item for pasting (often text) into responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  Paste,

  /// A standard "enter full screen" item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  EnterFullScreen,

  /// An item for minimizing the window with the standard system controls.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  Minimize,

  /// An item for instructing the app to zoom
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  Zoom,

  /// Represents a Separator
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  Separator,
}

impl MenuItem {
  /// Create new custom menu item.
  /// unique_menu_id is the unique ID for the menu item returned in the EventLoop `Event::MenuEvent(unique_menu_id)`
  pub fn new(title: &'static str) -> Self {
    MenuItem::Custom(CustomMenu {
      _id: MenuId::new(title),
      name: title,
      keyboard_accelerators: None,
    })
  }

  /// Assign keyboard shortcut to the menu action. Works only with `MenuItem::Custom`.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Linux / Android / iOS:** Unsupported (noop).
  ///
  pub fn with_accelerators(mut self, keyboard_accelerators: &'static str) -> Self {
    if let MenuItem::Custom(ref mut custom_menu) = self {
      custom_menu.keyboard_accelerators = Some(keyboard_accelerators);
    }
    self
  }

  /// Return unique menu ID. Works only with `MenuItem::Custom`.
  pub fn id(mut self) -> MenuId {
    if let MenuItem::Custom(ref mut custom_menu) = self {
      return custom_menu._id;
    }

    // return blank menu id if we request under a non-custom menu
    // this prevent to wrap it inside an Option<>
    MenuId { 0: 4294967295 }
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
  fn new<T: Into<String>>(menu_title: T) -> MenuId {
    MenuId(hash_string_to_u32(menu_title.into()))
  }
}

/// Type of menu the click is originating from.
#[derive(Clone, Debug, PartialEq)]
pub enum MenuType {
  /// Menubar menu item.
  Menubar,
  /// Statusbar menu item.
  Statusbar,
}

fn hash_string_to_u32(title: String) -> u32 {
  let mut s = DefaultHasher::new();
  title.hash(&mut s);
  s.finish() as u32
}
