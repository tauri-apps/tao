use std::sync::mpsc::Sender;

use gtk::{prelude::*, ApplicationWindow, Menu, MenuBar, MenuItem, Orientation};

use super::window::{WindowId, WindowRequest};
use crate::menu::Menu as RootMenu;

pub fn initialize(
  window: &ApplicationWindow,
  menus: Vec<RootMenu>,
  tx: Sender<(WindowId, WindowRequest)>,
) {
  let id = WindowId(window.get_id());
  let vbox = gtk::Box::new(Orientation::Vertical, 0);
  let menubar = MenuBar::new();
  window.add(&vbox);

  for menu in menus {
    let title = MenuItem::with_label(&menu.title);
    let submenu = Menu::new();

    for i in menu.items {
      let item = match i {
        // TODO other MenuItem variant
        _ => {
          let item = MenuItem::with_label("Quit");
          let tx_ = tx.clone();
          item.connect_activate(move |_| {
            if let Err(e) = tx_.send((id, WindowRequest::Close)) {
              log::warn!("Fail to send close window request: {}", e);
            }
          });
          item
        }
      };

      submenu.append(&item);
    }

    title.set_submenu(Some(&submenu));
    menubar.append(&title);
  }

  vbox.pack_start(&menubar, false, false, 0);
}
