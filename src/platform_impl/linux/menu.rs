// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use glib::Sender;
use gtk::{
  prelude::*, AccelFlags, AccelGroup, Menu as GtkMenu, MenuBar, MenuItem as GtkMenuItem,
  SeparatorMenuItem,
};

use super::window::{WindowId, WindowRequest};
use crate::menu::{MenuIcon, MenuId, MenuItem, MenuType};

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

#[derive(Debug, Clone)]
pub struct CustomMenuItem {
  pub id: MenuId,
  title: String,
  key: Option<String>,
  enabled: bool,
  selected: bool,
  gtk_item: GtkMenuItem,
}

impl CustomMenuItem {
  pub fn set_enabled(&mut self, is_enabled: bool) {
    self.gtk_item.set_sensitive(is_enabled);
  }
  pub fn set_title(&mut self, title: &str) {
    self.gtk_item.set_label(title);
  }
  pub fn set_selected(&mut self, is_selected: bool) {}
  pub fn set_icon(&mut self, icon: MenuIcon) {}
}

impl Default for Menu {
  fn default() -> Self {
    Menu::new()
  }
}

impl Menu {
  pub fn new() -> Self {
    Menu {
      gtk_items: Vec::new(),
    }
  }
  pub fn new_popup_menu() -> Self {
    Self::new()
  }

  pub fn add_item(&mut self, item: MenuItem, _menu_type: MenuType) -> Option<CustomMenuItem> {
    self.gtk_items.push(item);
    None
  }
  pub fn add_custom_item(
    &mut self,
    id: MenuId,
    _menu_type: MenuType,
    text: &str,
    key: Option<&str>,
    enabled: bool,
    selected: bool,
  ) -> CustomMenuItem {
    let custom_item = CustomMenuItem {
      title: text.to_string(),
      id,
      key: key.map(String::from),
      enabled,
      selected,
      gtk_item: GtkMenuItem::with_label(&text),
    };
    let item = MenuItem::Custom(custom_item.clone());

    self.gtk_items.push(item.clone());

    custom_item
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
    for menu_item in self.gtk_items {
      let new_item = match menu_item.clone() {
        MenuItem::Submenu(title, _enabled, submenu) => {
          // FIXME: enabled is not used here
          let item = GtkMenuItem::with_label(&title);
          item.set_submenu(Some(&submenu.into_gtkmenu(tx, accel_group, window_id)));
          Some(item)
        }
        MenuItem::Custom(custom) => {
          if let Some(key) = custom.key {
            let (key, mods) = gtk::accelerator_parse(&key);
            custom.gtk_item.add_accelerator(
              "activate",
              accel_group,
              key,
              mods,
              AccelFlags::VISIBLE,
            );
          }

          // todo enabled
          if custom.enabled {}
          // todo selected
          if custom.selected {}

          let tx_ = tx.clone();
          custom.gtk_item.connect_activate(move |_| {
            if let Err(e) = tx_.send((window_id, WindowRequest::Menu(menu_item.clone()))) {
              log::warn!("Fail to send menu request: {}", e);
            }
          });

          Some(custom.gtk_item)
        }
        MenuItem::Separator => {
          menu.append(&SeparatorMenuItem::new());
          None
        }
        MenuItem::About(s) => Some(GtkMenuItem::with_label(&format!("About {}", s))),
        MenuItem::Hide => menuitem!("Hide", "<Ctrl>H", accel_group),
        MenuItem::CloseWindow => menuitem!("Close Window", "<Ctrl>W", accel_group),
        MenuItem::Quit => menuitem!("Quit", "Q", accel_group),
        MenuItem::Copy => menuitem!("Copy", "<Ctrl>C", accel_group),
        MenuItem::Cut => menuitem!("Cut", "<Ctrl>X", accel_group),
        MenuItem::SelectAll => menuitem!("Select All", "<Ctrl>A", accel_group),
        MenuItem::Paste => menuitem!("Paste", "<Ctrl>V", accel_group),
        // todo add others
        _ => None,
      };

      if let Some(new_item) = new_item {
        menu.append(&new_item);
      }
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
