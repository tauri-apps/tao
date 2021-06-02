// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use glib::Sender;
use gtk::{
  prelude::*, AccelFlags, AccelGroup, Menu as GtkMenu, MenuBar, MenuItem as GtkMenuItem,
  SeparatorMenuItem,
};

use super::window::{WindowId, WindowRequest};
use crate::menu::{CustomMenuItem as RootCustomMenuItem, MenuId, MenuItem, MenuType};

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
  gtk_items: Vec<(MenuItem, Option<CustomMenuItem>)>,
}

unsafe impl Send for Menu {}
unsafe impl Sync for Menu {}

#[derive(Debug, Clone)]
pub struct CustomMenuItem {
  id: MenuId,
  gtk_item: GtkMenuItem,
}

impl CustomMenuItem {
  pub fn id(self) -> MenuId {
    self.id
  }
  pub fn set_enabled(&mut self, is_enabled: bool) {
    self.gtk_item.set_sensitive(is_enabled);
  }
  pub fn set_title(&mut self, title: &str) {
    self.gtk_item.set_label(title);
  }

  // todo's
  pub fn set_selected(&mut self, _is_selected: bool) {}
  pub fn set_icon(&mut self, _icon: Vec<u8>) {}
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

  pub fn add_item(&mut self, item: MenuItem, _menu_type: MenuType) -> Option<RootCustomMenuItem> {
    if let MenuItem::Custom { text, menu_id, .. } = item.clone() {
      let new_gtk_item = GtkMenuItem::with_label(&text);
      let custom_menu = CustomMenuItem {
        gtk_item: new_gtk_item,
        id: menu_id,
      };

      self
        .gtk_items
        .push((item.clone(), Some(custom_menu.clone())));
      return Some(RootCustomMenuItem(custom_menu));
    }

    self.gtk_items.push((item, None));

    None
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
        (
          MenuItem::Submenu {
            enabled,
            menu_platform,
            title,
          },
          _,
        ) => {
          // FIXME: enabled is not used here
          let item = GtkMenuItem::with_label(&title);
          item.set_submenu(Some(&menu_platform.into_gtkmenu(
            tx,
            accel_group,
            window_id,
          )));
          Some(item)
        }
        (
          MenuItem::Custom {
            keyboard_accelerator,
            enabled,
            selected,
            ..
          },
          Some(custom_item),
        ) => {
          let gtk_item = custom_item.gtk_item;
          if let Some(key) = keyboard_accelerator {
            let (key, mods) = gtk::accelerator_parse(&key);
            gtk_item.add_accelerator("activate", accel_group, key, mods, AccelFlags::VISIBLE);
          }

          gtk_item.set_sensitive(enabled);

          // todo selected
          if selected {}

          let tx_ = tx.clone();
          gtk_item.connect_activate(move |_| {
            if let Err(e) = tx_.send((window_id, WindowRequest::Menu(menu_item.clone().0))) {
              log::warn!("Fail to send menu request: {}", e);
            }
          });

          Some(gtk_item)
        }
        (MenuItem::Separator, _) => {
          menu.append(&SeparatorMenuItem::new());
          None
        }
        (MenuItem::About(s), _) => Some(GtkMenuItem::with_label(&format!("About {}", s))),
        (MenuItem::Hide, _) => menuitem!("Hide", "<Ctrl>H", accel_group),
        (MenuItem::CloseWindow, _) => menuitem!("Close Window", "<Ctrl>W", accel_group),
        (MenuItem::Quit, _) => menuitem!("Quit", "Q", accel_group),
        (MenuItem::Copy, _) => menuitem!("Copy", "<Ctrl>C", accel_group),
        (MenuItem::Cut, _) => menuitem!("Cut", "<Ctrl>X", accel_group),
        (MenuItem::SelectAll, _) => menuitem!("Select All", "<Ctrl>A", accel_group),
        (MenuItem::Paste, _) => menuitem!("Paste", "<Ctrl>V", accel_group),
        // todo add others
        (_, _) => None,
      };

      if let Some(new_item) = new_item {
        menu.append(&new_item);
      }
    }
  }
}

// Generate menu for menu bar.
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
