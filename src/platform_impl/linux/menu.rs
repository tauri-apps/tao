// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use glib::Sender;
use gtk::{
  prelude::*, AccelFlags, AccelGroup, Menu as GtkMenu, MenuBar, MenuItem as GtkMenuItem,
  SeparatorMenuItem,
};

use super::window::{WindowId, WindowRequest};
use crate::menu::{MenuIcon, MenuId, MenuType, MenuAction};

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

pub struct CustomMenuItem {
  id: MenuId,
  title: String,
  key: Option<String>,
  enabled: bool,
  gtk_item: GtkMenuItem,
}

impl CustomMenuItem {
  pub fn set_enabled(&mut self, is_enabled: bool) {
    if let Self::Custom { gtk_item, .. } = self {
      gtk_item.set_sensitive(is_enabled);
    }
  }
  pub fn set_title(&mut self, title: &str) {
    if let Self::Custom { gtk_item, .. } = self {
      gtk_item.set_label(title);
    }
  }
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
  pub fn add_system_item(&mut self, item: MenuAction, menu_type: MenuType) -> Option<MenuItem> {
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
      gtk_item: GtkMenuItem::with_label(&text),
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
          key,
          gtk_item,
          ..
        } => {
          if let Some(key) = key {
            let (key, mods) = gtk::accelerator_parse(&key);
            gtk_item.add_accelerator("activate", accel_group, key, mods, AccelFlags::VISIBLE);
          }

          // todo enabled
          if enabled {}

          let tx_ = tx.clone();
          gtk_item.connect_activate(move |_| {
            if let Err(e) = tx_.send((window_id, WindowRequest::Menu(menu_item.clone()))) {
              log::warn!("Fail to send menu request: {}", e);
            }
          });

          menu.append(&gtk_item);
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
