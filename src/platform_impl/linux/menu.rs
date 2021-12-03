// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use glib::{Cast, Sender};
use gtk::{
  prelude::*, AccelFlags, AccelGroup, CheckMenuItem, Menu as GtkMenu, MenuItem as GtkMenuItem,
  SeparatorMenuItem,
};

use super::{
  keyboard::key_to_raw_key,
  window::{WindowId, WindowRequest},
};
use crate::{
  accelerator::Accelerator,
  keyboard::{KeyCode, ModifiersState},
  menu::{CustomMenuItem, MenuId, MenuItem, MenuType},
};

macro_rules! menuitem {
  ( $description:expr, $key:expr, $accel_group:ident, $window_id:expr, $native_menu_item:expr, $tx:ident ) => {{
    let item = GtkMenuItem::with_label($description);
    let (key, mods) = gtk::accelerator_parse($key);
    item.add_accelerator("activate", $accel_group, key, mods, AccelFlags::VISIBLE);
    item.connect_activate(move |_| {
      if let Err(e) = $tx.send(($window_id, WindowRequest::Menu(($native_menu_item, None)))) {
        log::warn!("Fail to send native menu request: {}", e);
      }
    });
    Some(item)
  }};
}

#[derive(Debug, Clone)]
struct GtkMenuInfo {
  menu_type: GtkMenuType,
  menu_item: Option<MenuItem>,
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
#[allow(dead_code)]
pub struct MenuItemAttributes {
  id: MenuId,
  key: Option<Accelerator>,
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
    accelerators: Option<Accelerator>,
    enabled: bool,
    selected: bool,
    menu_type: MenuType,
  ) -> CustomMenuItem {
    let gtk_item = if selected {
      let item = CheckMenuItem::with_label(title);
      item.set_active(true);
      item.upcast::<GtkMenuItem>()
    } else {
      GtkMenuItem::with_label(title)
    };
    let custom_menu = MenuItemAttributes {
      id: menu_id,
      key: accelerators,
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
    item: MenuItem,
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

  pub(crate) fn generate_menu<M: gtk::prelude::IsA<gtk::MenuShell>>(
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
          sub_menu:
            Some(SubmenuDetail {
              menu,
              title,
              enabled,
              ..
            }),
          ..
        } => {
          let item = GtkMenuItem::with_label(&title);
          item.set_sensitive(enabled);
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
              ..
            }),
          ..
        } => {
          if let Some(key) = key {
            register_accelerator(&gtk_item, accel_group, key);
          }

          gtk_item.set_sensitive(enabled);

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
          menu_item: Some(MenuItem::Separator),
          ..
        } => {
          menu.append(&SeparatorMenuItem::new());
          None
        }
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(MenuItem::About(s)),
          ..
        } => Some(GtkMenuItem::with_label(&format!("About {}", s))),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(MenuItem::Hide),
          ..
        } => {
          let tx_clone = tx.clone();
          menuitem!(
            "Hide",
            "<Ctrl>H",
            accel_group,
            window_id,
            Some(MenuItem::Hide),
            tx_clone
          )
        }
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(MenuItem::CloseWindow),
          ..
        } => {
          let tx_clone = tx.clone();
          menuitem!(
            "Close Window",
            "<Ctrl>W",
            accel_group,
            window_id,
            Some(MenuItem::CloseWindow),
            tx_clone
          )
        }
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(MenuItem::Quit),
          ..
        } => {
          let tx_clone = tx.clone();
          menuitem!(
            "Quit",
            "<Ctrl>Q",
            accel_group,
            window_id,
            Some(MenuItem::Quit),
            tx_clone
          )
        }
        // TODO add others
        _ => None,
      };

      if let Some(new_item) = new_item {
        menu.append(&new_item);
      }
    }
  }
}

fn register_accelerator(item: &GtkMenuItem, accel_group: &AccelGroup, menu_key: Accelerator) {
  let accel_key = match &menu_key.key {
    KeyCode::KeyA => 'A' as u32,
    KeyCode::KeyB => 'B' as u32,
    KeyCode::KeyC => 'C' as u32,
    KeyCode::KeyD => 'D' as u32,
    KeyCode::KeyE => 'E' as u32,
    KeyCode::KeyF => 'F' as u32,
    KeyCode::KeyG => 'G' as u32,
    KeyCode::KeyH => 'H' as u32,
    KeyCode::KeyI => 'I' as u32,
    KeyCode::KeyJ => 'J' as u32,
    KeyCode::KeyK => 'K' as u32,
    KeyCode::KeyL => 'L' as u32,
    KeyCode::KeyM => 'M' as u32,
    KeyCode::KeyN => 'N' as u32,
    KeyCode::KeyO => 'O' as u32,
    KeyCode::KeyP => 'P' as u32,
    KeyCode::KeyQ => 'Q' as u32,
    KeyCode::KeyR => 'R' as u32,
    KeyCode::KeyS => 'S' as u32,
    KeyCode::KeyT => 'T' as u32,
    KeyCode::KeyU => 'U' as u32,
    KeyCode::KeyV => 'V' as u32,
    KeyCode::KeyW => 'W' as u32,
    KeyCode::KeyX => 'X' as u32,
    KeyCode::KeyY => 'Y' as u32,
    KeyCode::KeyZ => 'Z' as u32,
    KeyCode::Digit0 => '0' as u32,
    KeyCode::Digit1 => '1' as u32,
    KeyCode::Digit2 => '2' as u32,
    KeyCode::Digit3 => '3' as u32,
    KeyCode::Digit4 => '4' as u32,
    KeyCode::Digit5 => '5' as u32,
    KeyCode::Digit6 => '6' as u32,
    KeyCode::Digit7 => '7' as u32,
    KeyCode::Digit8 => '8' as u32,
    KeyCode::Digit9 => '9' as u32,
    KeyCode::Comma => ',' as u32,
    KeyCode::Minus => '-' as u32,
    KeyCode::Period => '.' as u32,
    KeyCode::Space => ' ' as u32,
    KeyCode::Equal => '=' as u32,
    KeyCode::Semicolon => ';' as u32,
    KeyCode::Slash => '/' as u32,
    KeyCode::Backslash => '\\' as u32,
    KeyCode::Quote => '\'' as u32,
    KeyCode::Backquote => '`' as u32,
    KeyCode::BracketLeft => '[' as u32,
    KeyCode::BracketRight => ']' as u32,
    k => {
      if let Some(gdk_key) = key_to_raw_key(k) {
        *gdk_key
      } else {
        dbg!("Cannot map key {:?}", k);
        return;
      }
    }
  };

  item.add_accelerator(
    "activate",
    accel_group,
    accel_key,
    modifiers_to_gdk_modifier_type(menu_key.mods),
    gtk::AccelFlags::VISIBLE,
  );
}

fn modifiers_to_gdk_modifier_type(modifiers: ModifiersState) -> gdk::ModifierType {
  let mut result = gdk::ModifierType::empty();

  result.set(gdk::ModifierType::MOD1_MASK, modifiers.alt_key());
  result.set(gdk::ModifierType::CONTROL_MASK, modifiers.control_key());
  result.set(gdk::ModifierType::SHIFT_MASK, modifiers.shift_key());
  result.set(gdk::ModifierType::META_MASK, modifiers.super_key());

  result
}
