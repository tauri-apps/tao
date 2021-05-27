// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use glib::Sender;
use gtk::{
  prelude::*, AccelFlags, AccelGroup, Menu as GtkMenu, MenuBar, MenuItem as GtkMenuItem,
  SeparatorMenuItem,
};

use super::window::{WindowId, WindowRequest};
use crate::menu::{MenuIcon, MenuId, MenuType, SystemMenu};

macro_rules! menuitem {
  ( $description:expr, $key:expr, $accel_group:ident ) => {{
    let item = GtkMenuItem::with_label($description);
    let (key, mods) = gtk::accelerator_parse($key);
    item.add_accelerator("activate", $accel_group, key, mods, AccelFlags::VISIBLE);
    Some(item)
  }};
}

#[derive(Debug, Clone)]
pub struct Menu {
  gtk_items: Vec<MenuItem>,
}

unsafe impl Send for Menu {}
unsafe impl Sync for Menu {}

// in linux we need to build our
// menu and then generate it in the initialization
#[derive(Debug, Clone)]
pub enum MenuItem {
  Custom {
    id: MenuId,
    title: String,
    key: Option<String>,
    enabled: bool,
  },
  Children(String, Menu),
  Separator,
  // todo add other elements
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
}

impl MenuItem {
  pub fn set_enabled(&mut self, is_enabled: bool) {}
  pub fn set_title(&mut self, title: &str) {}
  pub fn set_selected(&mut self, is_selected: bool) {}
  pub fn set_icon(&mut self, icon: MenuIcon) {}
}

// initialize menu and allocate the ID
impl Menu {
  pub fn new() -> Self {
    Menu {
      gtk_items: Vec::new(),
    }
  }
  pub fn new_popup_menu() -> Self {
    Self::new()
  }
  pub fn add_separator(&mut self) {
    //self.menu.append(&SeparatorMenuItem::new());
    self.gtk_items.push(MenuItem::Separator)
  }
  pub fn add_children(&mut self, menu: Self, title: &str, enabled: bool) {
    //let item = MenuItem::with_label(&title);
    //item.set_submenu(Some(&menu.menu));
    //self.menu.append(&item);
    self
      .gtk_items
      .push(MenuItem::Children(title.to_string(), menu));
  }
  pub fn add_system_item(&mut self, item: SystemMenu, menu_type: MenuType) -> Option<MenuItem> {
    None
  }
  pub fn add_custom_item(
    &mut self,
    id: MenuId,
    menu_type: MenuType,
    text: &str,
    key: Option<&str>,
    enabled: bool,
    selected: bool,
  ) -> MenuItem {
    let item = MenuItem::Custom {
      title: text.to_string(),
      id,
      key: key.map(String::from),
      enabled,
    };
    self.gtk_items.push(item.clone());

    item
  }

  pub fn into_gtkmenu(
    self,
    tx: &Sender<(WindowId, WindowRequest)>,
    accel_group: &AccelGroup,
    window_id: WindowId,
  ) -> GtkMenu {
    let mut menu = GtkMenu::new();
    menu.set_accel_group(Some(accel_group));
    self.generate_menu(&mut menu, tx, accel_group, window_id);
    menu
  }

  fn generate_menu<M: gtk::prelude::IsA<gtk::MenuShell>>(
    self,
    menu: &mut M,
    tx: &Sender<(WindowId, WindowRequest)>,
    accel_group: &AccelGroup,
    window_id: WindowId,
  ) {
    let tx_ = tx.clone();

    for menu_item in self.gtk_items {
      match menu_item.clone() {
        MenuItem::Children(title, submenu) => {
          let item = GtkMenuItem::with_label(&title);
          item.set_submenu(Some(&submenu.into_gtkmenu(tx, accel_group, window_id)));
          menu.append(&item);
        }
        MenuItem::Custom {
          enabled,
          id,
          key,
          title,
        } => {
          let item = match key {
            Some(key) => menuitem!(&title, &key, accel_group),
            None => Some(GtkMenuItem::with_label(&title)),
          };

          if let Some(new_item) = item {
            let tx_ = tx.clone();
            new_item.connect_activate(move |_| {
              if let Err(e) = tx_.send((window_id, WindowRequest::Menu(menu_item.clone()))) {
                log::warn!("Fail to send menu request: {}", e);
              }
            });

            menu.append(&new_item);
          }
        }
        MenuItem::Separator => menu.append(&SeparatorMenuItem::new()),
        // todo add others
        _ => {}
      };
    }
  }
}

pub fn initialize(
  id: WindowId,
  menu: Menu,
  tx: &Sender<(WindowId, WindowRequest)>,
  accel_group: &AccelGroup,
) -> MenuBar {
  let mut menubar = MenuBar::new();
  let () = menu.generate_menu(&mut menubar, tx, accel_group, id);
  menubar
}
