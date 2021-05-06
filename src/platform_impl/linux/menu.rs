use std::sync::mpsc::Sender;

use gtk::{prelude::*, ApplicationWindow, Menu, MenuBar, MenuItem, Orientation};

use super::window::{WindowId, WindowRequest};
use crate::menu::{Menu as RootMenu, MenuItem as RootMenuItem};

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
      let item = match &i {
        RootMenuItem::Custom(m) => MenuItem::with_label(&m.name),
        RootMenuItem::About(s) => MenuItem::with_label(&format!("About {}", s)),
        RootMenuItem::Hide => MenuItem::with_label("Hide"),
        RootMenuItem::Services => MenuItem::with_label("Services"),
        RootMenuItem::HideOthers => MenuItem::with_label("Hide Others"),
        RootMenuItem::ShowAll => MenuItem::with_label("Show All"),
        RootMenuItem::CloseWindow => MenuItem::with_label("Close Window"),
        RootMenuItem::Quit => MenuItem::with_label("Quit"),
        RootMenuItem::Copy => MenuItem::with_label("Copy"),
        RootMenuItem::Cut => MenuItem::with_label("Cut"),
        RootMenuItem::Undo => MenuItem::with_label("Undo"),
        RootMenuItem::Redo => MenuItem::with_label("Redo"),
        RootMenuItem::SelectAll => MenuItem::with_label("Select All"),
        RootMenuItem::Paste => MenuItem::with_label("Paste"),
        RootMenuItem::Zoom => MenuItem::with_label("Zoom"),
        RootMenuItem::Separator => MenuItem::with_label("Separator"),
        _ => MenuItem::with_label("Close Window"),
      };

      let tx_ = tx.clone();
      item.connect_activate(move |_| {
        if let Err(e) = tx_.send((id, WindowRequest::Menu(i.clone()))) {
          log::warn!("Fail to send menu request: {}", e);
        }
      });

      submenu.append(&item);
    }

    title.set_submenu(Some(&submenu));
    menubar.append(&title);
  }

  vbox.pack_start(&menubar, false, false, 0);
}
