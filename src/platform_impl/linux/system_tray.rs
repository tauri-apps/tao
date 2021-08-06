// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{
  error::OsError, event_loop::EventLoopWindowTarget, system_tray::SystemTray as RootSystemTray,
};

use glib::Sender;
use std::path::PathBuf;

use gtk::{prelude::WidgetExt, AccelGroup};
#[cfg(not(feature = "ayatana"))]
use libappindicator::{AppIndicator, AppIndicatorStatus};
#[cfg(feature = "ayatana")]
use libayatana_appindicator::{AppIndicator, AppIndicatorStatus};

use super::{menu::Menu, window::WindowRequest, WindowId};

pub struct SystemTrayBuilder {
  tray_menu: Option<Menu>,
  app_indicator: AppIndicator,
}

impl SystemTrayBuilder {
  #[inline]
  pub fn new(icon: PathBuf, tray_menu: Option<Menu>) -> Self {
    let path = icon.parent().expect("Invalid icon");
    let app_indicator = AppIndicator::with_path(
      "tao application",
      &icon.to_string_lossy(),
      &path.to_string_lossy(),
    );

    Self {
      tray_menu,
      app_indicator,
    }
  }

  #[inline]
  pub fn build<T: 'static>(
    mut self,
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<RootSystemTray, OsError> {
    let sender = window_target.p.window_requests_tx.clone();

    if let Some(tray_menu) = self.tray_menu.clone() {
      let menu = &mut tray_menu.into_gtkmenu(&sender, &AccelGroup::new(), WindowId::dummy());

      self.app_indicator.set_menu(menu);
      menu.show_all();
    }

    self.app_indicator.set_status(AppIndicatorStatus::Active);

    Ok(RootSystemTray(SystemTray {
      app_indicator: self.app_indicator,
      sender,
    }))
  }
}

pub struct SystemTray {
  app_indicator: AppIndicator,
  sender: Sender<(WindowId, WindowRequest)>,
}

impl SystemTray {
  pub fn set_icon(&mut self, icon: PathBuf) {
    let path = icon.parent().expect("Invalid icon");
    self
      .app_indicator
      .set_icon_theme_path(&path.to_string_lossy());
    self.app_indicator.set_icon(&icon.to_string_lossy())
  }

  pub fn set_menu(&mut self, tray_menu: &Menu) {
    let mut menu =
      tray_menu
        .clone()
        .into_gtkmenu(&self.sender, &AccelGroup::new(), WindowId::dummy());

    self.app_indicator.set_menu(&mut menu);
    menu.show_all();
  }
}
