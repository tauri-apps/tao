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

#[derive(Debug, Clone)]
pub struct CustomMenu {
  // todo: replace id by a type that
  // we can send from the with_menu::<T>()
  // or we could check to use the UserEvent::<T> but
  // my latest test failed
  pub id: String,
  pub name: String,
  pub keyboard_accelerators: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub enum MenuItem {
  /// A custom MenuItem
  Custom(CustomMenu),

  /// Shows a standard "About" item
  About(&'static str),

  /// A standard "hide the app" menu item.
  Hide,

  /// A standard "Services" menu item.
  /// ## Platform-specific
  /// macOS only.
  Services,

  /// A "hide all other windows" menu item.
  HideOthers,

  /// A menu item to show all the windows for this app.
  ShowAll,

  /// Close the current window.
  CloseWindow,

  /// A "quit this app" menu icon.
  Quit,

  /// A menu item for enabling copying (often text) from responders.
  Copy,

  /// A menu item for enabling cutting (often text) from responders.
  Cut,

  /// An "undo" menu item; particularly useful for supporting the cut/copy/paste/undo lifecycle
  /// of events.
  Undo,

  /// An "redo" menu item; particularly useful for supporting the cut/copy/paste/undo lifecycle
  /// of events.
  Redo,

  /// A menu item for selecting all (often text) from responders.
  SelectAll,

  /// A menu item for pasting (often text) into responders.
  Paste,

  /// A standard "enter full screen" item.
  EnterFullScreen,

  /// An item for minimizing the window with the standard system controls.
  Minimize,

  /// An item for instructing the app to zoom
  Zoom,

  /// Represents a Separator
  Separator,
}

impl MenuItem {
  /// Create new custom menu item.
  /// unique_menu_id is the unique ID for the menu item returned in the EventLoop `Event::MenuEvent(unique_menu_id)`
  pub fn new<T>(unique_menu_id: T, title: T) -> Self
  where
    T: Into<String>,
  {
    MenuItem::Custom(CustomMenu {
      // todo: would be great if we could pass a type instead of an ID?
      id: unique_menu_id.into(),
      name: title.into(),
      keyboard_accelerators: None,
    })
  }

  /// Assign keyboard shortcut
  pub fn with_accelerators(mut self, keyboard_accelerators: &'static str) -> Self {
    if let MenuItem::Custom(ref mut custom_menu) = self {
      custom_menu.keyboard_accelerators = Some(keyboard_accelerators);
    }
    self
  }
}
