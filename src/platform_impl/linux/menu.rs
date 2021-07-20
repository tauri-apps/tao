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
  ( $description:expr, $key:expr, $accel_group:ident ) => {{
    let item = GtkMenuItem::with_label($description);
    let (key, mods) = gtk::accelerator_parse($key);
    item.add_accelerator("activate", $accel_group, key, mods, AccelFlags::VISIBLE);
    Some(item)
  }};
}

macro_rules! ksni_menuitem {
  ( $description:expr ) => {
    ksni::menu::StandardItem {
      label: $description.into(),
      ..Default::default()
    }
    .into()
  };
  ( $description:expr, $modifier:expr, $key:expr ) => {
    ksni::menu::StandardItem {
      label: $description.into(),
      shortcut: vec![vec![$modifier.into(), $key.into()]],
      ..Default::default()
    }
    .into()
  };
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
pub struct MenuItemAttributes {
  id: MenuId,
  key: Option<Accelerator>,
  title: String,
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
    self.title = title.into();
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
      let item = CheckMenuItem::with_label(&title);
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
      title: title.into(),
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

  #[cfg(feature = "tray")]
  pub fn to_ksnimenu(
    &self,
    tx: &Sender<(WindowId, WindowRequest)>,
    window_id: WindowId,
  ) -> Vec<ksni::MenuItem<super::system_tray::KsniTray>> {
    let mut v = vec![];
    for item in &self.gtk_items {
      if let Some(m) = Self::generate_ksnimenu_item(item, tx.clone(), window_id) {
        v.push(m);
      }
      println!("{:#?}", item);
    }
    v
  }

  #[cfg(feature = "tray")]
  fn generate_ksnimenu_item(
    item: &GtkMenuInfo,
    tx: Sender<(WindowId, WindowRequest)>,
    window_id: WindowId,
  ) -> Option<ksni::menu::MenuItem<super::system_tray::KsniTray>> {
    match item {
      GtkMenuInfo {
        menu_type: GtkMenuType::Custom,
        sub_menu: None,
        custom_menu_item: Some(i),
        ..
      } => {
        let id = i.id;

        let ksni_accelerator = if let Some(a) = &i.key {
          super::system_tray::KsniAccelerator::from(a).into_vec()
        } else {
          vec![]
        };

        let ksni_item = if i.selected {
          ksni::menu::CheckmarkItem {
            label: i.title.clone(),
            enabled: i.enabled,
            visible: true,
            activate: Box::new(move |_| {
              if let Err(e) = tx.send((window_id, WindowRequest::Menu((None, Some(id))))) {
                log::warn!("Fail to send menu request: {}", e);
              }
            }),
            shortcut: vec![ksni_accelerator],
            ..Default::default()
          }
          .into()
        } else {
          ksni::menu::StandardItem {
            label: i.title.clone(),
            enabled: i.enabled,
            visible: true,
            activate: Box::new(move |_| {
              if let Err(e) = tx.send((window_id, WindowRequest::Menu((None, Some(id))))) {
                log::warn!("Fail to send menu request: {}", e);
              }
            }),
            shortcut: vec![ksni_accelerator],
            ..Default::default()
          }
          .into()
        };
        Some(ksni_item)
      }
      GtkMenuInfo {
        menu_type: GtkMenuType::Submenu,
        sub_menu:
          Some(SubmenuDetail {
            menu,
            title,
            enabled,
            ..
          }),
        custom_menu_item: None,
        ..
      } => {
        let mut submenu = vec![];
        for item in &menu.gtk_items {
          if let Some(i) = Self::generate_ksnimenu_item(&item, tx.clone(), window_id) {
            submenu.push(i);
          }
        }
        let ksni_item = ksni::menu::SubMenu {
          label: title.clone(),
          enabled: *enabled,
          visible: true,
          submenu,
          ..Default::default()
        }
        .into();
        Some(ksni_item)
      }
      GtkMenuInfo {
        menu_type: GtkMenuType::Native,
        menu_item: Some(MenuItem::Separator),
        ..
      } => Some(ksni::MenuItem::Sepatator),
      GtkMenuInfo {
        menu_type: GtkMenuType::Native,
        menu_item: Some(MenuItem::About(s)),
        ..
      } => Some(ksni_menuitem!(&format!("About {}", s))),
      GtkMenuInfo {
        menu_type: GtkMenuType::Native,
        menu_item: Some(MenuItem::Hide),
        ..
      } => Some(ksni_menuitem!("Hide", "Control", "H")),
      GtkMenuInfo {
        menu_type: GtkMenuType::Native,
        menu_item: Some(MenuItem::CloseWindow),
        ..
      } => Some(ksni_menuitem!("Close Window", "Control", "W")),
      GtkMenuInfo {
        menu_type: GtkMenuType::Native,
        menu_item: Some(MenuItem::Quit),
        ..
      } => Some(ksni_menuitem!("Quit", "", "Q")),
      GtkMenuInfo {
        menu_type: GtkMenuType::Native,
        menu_item: Some(MenuItem::Copy),
        ..
      } => Some(ksni_menuitem!("Copy", "Control", "C")),
      GtkMenuInfo {
        menu_type: GtkMenuType::Native,
        menu_item: Some(MenuItem::Cut),
        ..
      } => Some(ksni_menuitem!("Cut", "Control", "X")),
      GtkMenuInfo {
        menu_type: GtkMenuType::Native,
        menu_item: Some(MenuItem::SelectAll),
        ..
      } => Some(ksni_menuitem!("Select All", "Control", "A")),
      GtkMenuInfo {
        menu_type: GtkMenuType::Native,
        menu_item: Some(MenuItem::Paste),
        ..
      } => Some(ksni_menuitem!("Paste", "Control", "V")),
      _ => {
        log::error!("Wrong combination of GtkMenuInfo");
        None
      }
    }
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
        } => menuitem!("Hide", "<Ctrl>H", accel_group),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(MenuItem::CloseWindow),
          ..
        } => menuitem!("Close Window", "<Ctrl>W", accel_group),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(MenuItem::Quit),
          ..
        } => menuitem!("Quit", "Q", accel_group),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(MenuItem::Copy),
          ..
        } => menuitem!("Copy", "<Ctrl>C", accel_group),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(MenuItem::Cut),
          ..
        } => menuitem!("Cut", "<Ctrl>X", accel_group),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(MenuItem::SelectAll),
          ..
        } => menuitem!("Select All", "<Ctrl>A", accel_group),
        GtkMenuInfo {
          menu_type: GtkMenuType::Native,
          menu_item: Some(MenuItem::Paste),
          ..
        } => menuitem!("Paste", "<Ctrl>V", accel_group),
        // todo add others
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
