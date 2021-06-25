// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{
  error::OsError, event_loop::EventLoopWindowTarget, system_tray::SystemTray as RootSystemTray,
};

use std::path::PathBuf;

use gtk::{AccelGroup, WidgetExt};
use libappindicator::{AppIndicator, AppIndicatorStatus};

use super::{menu::Menu, WindowId};

pub struct SystemTrayBuilder {
  pub(crate) system_tray: SystemTray,
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
      system_tray: SystemTray {
        tray_menu,
        app_indicator,
      },
    }
  }

  #[inline]
  pub fn build<T: 'static>(
    mut self,
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<RootSystemTray, OsError> {
    let tx_ = window_target.p.window_requests_tx.clone();

    if let Some(tray_menu) = self.system_tray.tray_menu.clone() {
      let menu = &mut tray_menu.into_gtkmenu(&tx_, &AccelGroup::new(), WindowId::dummy());

      self.system_tray.app_indicator.set_menu(menu);
      menu.show_all();
    }

    self
      .system_tray
      .app_indicator
      .set_status(AppIndicatorStatus::Active);

    Ok(RootSystemTray(self.system_tray))
  }
}

pub struct SystemTray {
  tray_menu: Option<Menu>,
  app_indicator: AppIndicator,
}

impl SystemTray {
  pub fn set_icon(&mut self, icon: PathBuf) {
    let path = icon.parent().expect("Invalid icon");
    self
      .app_indicator
      .set_icon_theme_path(&path.to_string_lossy());
    self.app_indicator.set_icon(&icon.to_string_lossy())
  }

  pub fn set_menu<T: 'static>(
    &mut self,
    tray_menu: Menu,
    window_target: &EventLoopWindowTarget<T>,
  ) {
    let tx_ = window_target.p.window_requests_tx.clone();
    let menu = &mut tray_menu.into_gtkmenu(&tx_, &AccelGroup::new(), WindowId::dummy());

    self.app_indicator.set_menu(menu);
    menu.show_all();
  }
}
