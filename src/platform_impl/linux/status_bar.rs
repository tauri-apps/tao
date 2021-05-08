use crate::{
  error::OsError, menu::MenuItem, platform::status_bar::Statusbar as RootStatusbar,
  platform_impl::EventLoopWindowTarget,
};
use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};

use super::window::{WindowId, WindowRequest};

pub struct Statusbar {}

impl Statusbar {
  pub(crate) fn initialize<T>(
    window_target: &EventLoopWindowTarget<T>,
    status_bar: &RootStatusbar,
  ) -> Result<(), OsError> {
    let icon = match status_bar.icon.file_stem() {
      Some(name) => name.to_string_lossy(),
      None => return Err(OsError::new(16, "status bar icon", super::OsError)),
    };
    let path = match status_bar.icon.parent() {
      Some(name) => name.to_string_lossy(),
      None => return Err(OsError::new(20, "status bar icon", super::OsError)),
    };
    let mut indicator = AppIndicator::with_path("tao application", &icon, &path);
    indicator.set_status(AppIndicatorStatus::Active);
    let mut m = gtk::Menu::new();

    for i in status_bar.items.iter() {
      match i {
        MenuItem::Custom(c) => {
          let item = gtk::MenuItem::with_label(&c.name);
          let tx_ = window_target.window_requests_tx.clone();
          let request = i.clone();
          item.connect_activate(move |_| {
            if let Err(e) = tx_.send((WindowId::dummy(), WindowRequest::Menu(request.clone()))) {
              log::warn!("Fail to send menu request: {}", e);
            }
          });
          m.append(&item);
        }
        MenuItem::Separator => {
          let item = gtk::SeparatorMenuItem::new();
          m.append(&item);
        }
        _ => (),
      }
    }

    indicator.set_menu(&mut m);
    m.show_all();
    Ok(())
  }
}
