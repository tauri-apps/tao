// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{
  error::OsError, platform::system_tray::SystemTray as RootSystemTray,
  platform_impl::EventLoopWindowTarget,
};

pub struct SystemTray {}

impl SystemTray {
  #[cfg(feature = "menu")]
  pub(crate) fn initialize<T>(
    window_target: &EventLoopWindowTarget<T>,
    system_tray: &RootSystemTray,
  ) -> Result<(), OsError> {
    use crate::menu::MenuItem;
    use gtk::prelude::*;
    use libappindicator::{AppIndicator, AppIndicatorStatus};

    use super::window::{WindowId, WindowRequest};

    let icon = match system_tray.icon.file_stem() {
      Some(name) => name.to_string_lossy(),
      None => return Err(OsError::new(16, "system tray icon", super::OsError)),
    };
    let path = match system_tray.icon.parent() {
      Some(name) => name.to_string_lossy(),
      None => return Err(OsError::new(20, "system tray icon", super::OsError)),
    };
    let mut indicator = AppIndicator::with_path("tao application", &icon, &path);
    indicator.set_status(AppIndicatorStatus::Active);
    let mut m = gtk::Menu::new();

    for i in system_tray.items.iter() {
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

  #[cfg(not(feature = "menu"))]
  pub(crate) fn initialize<T>(
    _window_target: &EventLoopWindowTarget<T>,
    _system_tray: &RootSystemTray,
  ) -> Result<(), OsError> {
    Ok(())
  }
}
