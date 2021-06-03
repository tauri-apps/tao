// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use glib::{Cast, Sender};
use gtk::{
  prelude::*, AccelFlags, AccelGroup, CheckMenuItem, Menu as GtkMenu, MenuBar,
  Menuitem as GtkMenuItem, SeparatorMenuItem,
};

use super::window::{WindowId, WindowRequest};
use crate::menu::{CustomMenuItem, MenuId, MenuType, Menuitem};

macro_rules! Menuitem {
  ( $description:expr, $key:expr, $accel_group:ident ) => {{
    let item = GtkMenuItem::with_label($description);
    let (key, mods) = gtk::accelerator_parse($key);
    item.add_accelerator("activate", $accel_group, key, mods, AccelFlags::VISIBLE);
    Some(item)
  }};
}

#[derive(Debug, Clone)]
struct GtkMenuInfo {
  menu_type: GtkMenuType,
  menu_item: Option<Menuitem>,
  sub_menu: Option<SubmenuDetail>,
  custom_menu_item: Option<MenuItemAttributes>,
}

#[derive(Debug, Clone)]
enum GtkMenuType {
  Custom,
  Submenu,
  Native,
}

#[derive(Debug, Clone)]
struct SubmenuDetail {
  menu: Menu,
  title: String,
  enabled: bool,
}

#[derive(Debug, Clone)]
pub struct Menu {
  gtk_items: Vec<GtkMenuInfo>,
}

unsafe impl Send for Menu {}
unsafe impl Sync for Menu {}

#[derive(Debug, Clone)]
pub struct MenuItemAttributes {
  id: MenuId,
  key: Option<String>,
  selected: bool,
  enabled: bool,
  menu_type: MenuType,
  gtk_item: GtkMenuItem,
}

impl MenuItemAttributes {
  pub fn id(self) -> MenuId {
    self.id
  }
  pub fn set_enabled(&mut self, is_enabled: bool) {
    self.gtk_item.set_sensitive(is_enabled);
  }
  pub fn set_title(&mut self, title: &str) {
    self.gtk_item.set_label(title);
  }

  pub fn set_selected(&mut self, is_selected: bool) {
    if let Some(item) = self.gtk_item.downcast_ref::<CheckMenuItem>() {
      item.set_active(is_selected);
    }
  }

  // TODO
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

  pub fn add_item(
    &mut self,
    menu_id: MenuId,
    title: &str,
    accelerators: Option<&str>,
    enabled: bool,
    selected: bool,
    menu_type: MenuType,
  ) -> CustomMenuItem {
    let gtk_item = if selected {
      let item = CheckMenuItem::with_label(&title);
      item.upcast::<GtkMenuItem>()
    } else {
      GtkMenuItem::with_label(&title)
    };
    let custom_menu = MenuItemAttributes {
      id: menu_id,
      key: accelerators.map(|a| a.to_string()),
      enabled,
      selected,
      menu_type,
      gtk_item,
    };

    self.gtk_items.push(GtkMenuInfo {
      menu_type: GtkMenuType::Custom,
      menu_item: None,
      sub_menu: None,
      custom_menu_item: Some(custom_menu.clone()),
    });
    CustomMenuItem(custom_menu)
  }

  pub fn add_native_item(
    &mut self,
    item: Menuitem,
    _menu_type: MenuType,
  ) -> Option<CustomMenuItem> {
    self.gtk_items.push(GtkMenuInfo {
      menu_type: GtkMenuType::Native,
      menu_item: Some(item),
      sub_menu: None,
      custom_menu_item: None,
    });
    None
  }

  pub fn add_submenu(&mut self, title: &str, enabled: bool, submenu: Menu) {
    self.gtk_items.push(GtkMenuInfo {
      menu_type: GtkMenuType::Submenu,
      menu_item: None,
      sub_menu: Some(SubmenuDetail {
        menu: submenu,
        title: title.to_string(),
        enabled,
      }),
      custom_menu_item: None,
    });
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
        GtkMenuInfo {
          menu_type: GtkMenuType::Submenu,
          sub_menu: Some(SubmenuDetail { menu, title, .. }),
          ..
        } => {
          // FIXME: enabled is not used here
          let item = GtkMenuItem::with_label(&title);
          item.set_submenu(Some(&menu.into_gtkmenu(tx, accel_group, window_id)));
          Some(item)
        }
        GtkMenuInfo {
          menu_type: GtkMenuType::Custom,
          custom_menu_item:
            Some(MenuItemAttributes {
              enabled,
              gtk_item,
              id,
              key,
              selected,
              ..
            }),
          ..
        } => {
          if let Some(key) = key {
            let (key, mods) = gtk::accelerator_parse(&key);
            gtk_item.add_accelerator("activate", accel_group, key, mods, AccelFlags::VISIBLE);
          }

          gtk_item.set_sensitive(enabled);

          // todo selected
          if selected {}

          let tx_ = tx.clone();
          gtk_item.connect_activate(move |_| {
            if let Err(e) = tx_.send((window_id, WindowRequest::Menu((None, Some(id))))) {
              log::warn!("Fail to send menu request: {}", e);
            }
          });

          Some(gtk_item)
        }
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(Menuitem::Separator),
          ..
        } => {
          menu.append(&SeparatorMenuItem::new());
          None
        }
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(Menuitem::About(s)),
          ..
        } => Some(GtkMenuItem::with_label(&format!("About {}", s))),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(Menuitem::Hide),
          ..
        } => Menuitem!("Hide", "<Ctrl>H", accel_group),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(Menuitem::CloseWindow),
          ..
        } => Menuitem!("Close Window", "<Ctrl>W", accel_group),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(Menuitem::Quit),
          ..
        } => Menuitem!("Quit", "Q", accel_group),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(Menuitem::Copy),
          ..
        } => Menuitem!("Copy", "<Ctrl>C", accel_group),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(Menuitem::Cut),
          ..
        } => Menuitem!("Cut", "<Ctrl>X", accel_group),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(Menuitem::SelectAll),
          ..
        } => Menuitem!("Select All", "<Ctrl>A", accel_group),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(Menuitem::Paste),
          ..
        } => Menuitem!("Paste", "<Ctrl>V", accel_group),
        // todo add others
        _ => None,
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
