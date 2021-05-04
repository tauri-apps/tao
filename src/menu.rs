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
/// CustomMenu is a custom menu who emit an event inside the EventLoop.
pub struct CustomMenu {
  // todo: replace id by a type that
  // we can send from the with_menu::<T>()
  // or we could check to use the UserEvent::<T> but
  // my latest test failed
  pub id: String,
  pub name: String,
  pub keyboard_accelerators: Option<&'static str>,
}

/// A menu item, binded to a pre-defined action or `Custom` emit an event.
#[derive(Debug, Clone)]
pub enum MenuItem {
  /// A custom menu emit an event inside the EventLoop.
  Custom(CustomMenu),

  /// Shows a standard "About" item
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  About(&'static str),

  /// A standard "hide the app" menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  Hide,

  /// A standard "Services" menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  Services,

  /// A "hide all other windows" menu item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  HideOthers,

  /// A menu item to show all the windows for this app.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  ShowAll,

  /// Close the current window.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  CloseWindow,

  /// A "quit this app" menu icon.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  Quit,

  /// A menu item for enabling copying (often text) from responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  Copy,

  /// A menu item for enabling cutting (often text) from responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  Cut,

  /// An "undo" menu item; particularly useful for supporting the cut/copy/paste/undo lifecycle
  /// of events.
  Undo,

  /// An "redo" menu item; particularly useful for supporting the cut/copy/paste/undo lifecycle
  /// of events.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  Redo,

  /// A menu item for selecting all (often text) from responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  SelectAll,

  /// A menu item for pasting (often text) into responders.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  Paste,

  /// A standard "enter full screen" item.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  EnterFullScreen,

  /// An item for minimizing the window with the standard system controls.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  Minimize,

  /// An item for instructing the app to zoom
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  Zoom,

  /// Represents a Separator
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
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

  /// Assign keyboard shortcut to the menu action. Works only with `MenuItem::Custom`.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows/Linux:** Unsupported (noop).
  ///
  pub fn with_accelerators(mut self, keyboard_accelerators: &'static str) -> Self {
    if let MenuItem::Custom(ref mut custom_menu) = self {
      custom_menu.keyboard_accelerators = Some(keyboard_accelerators);
    }
    self
  }
}
