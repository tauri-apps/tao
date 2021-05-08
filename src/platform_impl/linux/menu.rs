use std::sync::mpsc::Sender;

use gtk::{prelude::*, AccelFlags, AccelGroup, Menu, MenuBar, MenuItem, SeparatorMenuItem};

use super::window::{WindowId, WindowRequest};
use crate::menu::{Menu as RootMenu, MenuItem as RootMenuItem};

macro_rules! menuitem {
  ( $description:expr, $key:expr, $accel_group:ident ) => {{
    let item = MenuItem::with_label($description);
    let (key, mods) = gtk::accelerator_parse($key);
    item.add_accelerator("activate", $accel_group, key, mods, AccelFlags::VISIBLE);
    Some(item)
  }};
}

pub fn initialize(
  id: WindowId,
  menus: Vec<RootMenu>,
  tx: &Sender<(WindowId, WindowRequest)>,
  accel_group: &AccelGroup,
) -> MenuBar {
  let menubar = MenuBar::new();
  for menu in menus {
    let title = MenuItem::with_label(&menu.title);
    let submenu = Menu::new();

    for i in menu.items {
      let item = match &i {
        RootMenuItem::Custom(m) => match &m.keyboard_accelerators {
          Some(accel) => menuitem!(&m.name, &accel, accel_group),
          None => Some(MenuItem::with_label(&m.name)),
        },
        RootMenuItem::About(s) => Some(MenuItem::with_label(&format!("About {}", s))),
        RootMenuItem::Hide => menuitem!("Hide", "<Ctrl>H", accel_group),
        RootMenuItem::CloseWindow => menuitem!("Close Window", "<Ctrl>W", accel_group),
        RootMenuItem::Quit => menuitem!("Quit", "Q", accel_group),
        RootMenuItem::Copy => menuitem!("Copy", "<Ctrl>C", accel_group),
        RootMenuItem::Cut => menuitem!("Cut", "<Ctrl>X", accel_group),
        RootMenuItem::SelectAll => menuitem!("Select All", "<Ctrl>A", accel_group),
        RootMenuItem::Paste => menuitem!("Paste", "<Ctrl>V", accel_group),
        RootMenuItem::Separator => {
          let item = SeparatorMenuItem::new();
          submenu.append(&item);
          None
        }
        RootMenuItem::EnterFullScreen => menuitem!("Fullscreen", "F11", accel_group),
        RootMenuItem::Minimize => menuitem!("Minimize", "<Ctrl>M", accel_group),
        _ => None,
      };

      if let Some(item) = item {
        let tx_ = tx.clone();
        item.connect_activate(move |_| {
          if let Err(e) = tx_.send((id, WindowRequest::Menu(i.clone()))) {
            log::warn!("Fail to send menu request: {}", e);
          }
        });

        submenu.append(&item);
      }
    }

    title.set_submenu(Some(&submenu));
    menubar.append(&title);
  }

  menubar
}
