// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{
  error::OsError, event_loop::EventLoopWindowTarget, system_tray::SystemTray as RootSystemTray,
};

use glib::Sender;
use std::path::PathBuf;

use gtk::{prelude::WidgetExt, AccelGroup};

use libappindicator::{
  AppIndicator as GtkAppIndicator, AppIndicatorStatus as GtkAppIndicatorStatus,
};
use libayatana_appindicator::{
  AppIndicator as AyatanaAppIndicator, AppIndicatorStatus as AyatanaAppIndicatorStatus,
};

use super::{menu::Menu, window::WindowRequest, WindowId};

enum AppIndicatorStatus {
  Passive,
  Active,
  Attention,
}

impl From<AppIndicatorStatus> for GtkAppIndicatorStatus {
  fn from(s: AppIndicatorStatus) -> Self {
    match s {
      AppIndicatorStatus::Passive => GtkAppIndicatorStatus::Passive,
      AppIndicatorStatus::Active => GtkAppIndicatorStatus::Active,
      AppIndicatorStatus::Attention => GtkAppIndicatorStatus::Attention,
    }
  }
}
impl From<AppIndicatorStatus> for AyatanaAppIndicatorStatus {
  fn from(s: AppIndicatorStatus) -> Self {
    match s {
      AppIndicatorStatus::Passive => AyatanaAppIndicatorStatus::Passive,
      AppIndicatorStatus::Active => AyatanaAppIndicatorStatus::Active,
      AppIndicatorStatus::Attention => AyatanaAppIndicatorStatus::Attention,
    }
  }
}

trait AppIndicator {
  fn with_path(title: &str, icon: &str, theme_path: &str) -> Self
  where
    Self: Sized;
  fn set_icon(&mut self, name: &str);
  fn set_icon_theme_path(&mut self, path: &str);
  fn set_menu(&mut self, menu: &mut gtk::Menu);
  fn set_status(&mut self, status: AppIndicatorStatus);
}

impl AppIndicator for GtkAppIndicator {
  fn with_path(title: &str, icon: &str, theme_path: &str) -> Self
  where
    Self: Sized,
  {
    Self::with_path(title, icon, theme_path)
  }

  fn set_icon(&mut self, name: &str) {
    self.set_icon(name)
  }

  fn set_icon_theme_path(&mut self, path: &str) {
    self.set_icon_theme_path(path)
  }

  fn set_menu(&mut self, menu: &mut gtk::Menu) {
    self.set_menu(menu)
  }

  fn set_status(&mut self, status: AppIndicatorStatus) {
    self.set_status(status.into())
  }
}

impl AppIndicator for AyatanaAppIndicator {
  fn with_path(title: &str, icon: &str, theme_path: &str) -> Self
  where
    Self: Sized,
  {
    Self::with_path(title, icon, theme_path)
  }

  fn set_icon(&mut self, name: &str) {
    self.set_icon(name)
  }

  fn set_icon_theme_path(&mut self, path: &str) {
    self.set_icon_theme_path(path)
  }

  fn set_menu(&mut self, menu: &mut gtk::Menu) {
    self.set_menu(menu)
  }

  fn set_status(&mut self, status: AppIndicatorStatus) {
    self.set_status(status.into())
  }
}

pub struct SystemTrayBuilder {
  tray_menu: Option<Menu>,
  app_indicator: Box<dyn AppIndicator>,
}

fn probe_libayatana() -> bool {
  if let Err(_) = pkg_config::probe_library("ayatana-appindicator3-0.1") {
    return false;
  }
  return true;
}

fn probe_libindicator() -> bool {
  if let Err(_) = pkg_config::probe_library("appindicator3") {
    if let Err(_) = pkg_config::probe_library("appindicator3-0.1") {
      return false;
    }
  }
  return true;
}

impl SystemTrayBuilder {
  #[inline]
  pub fn new(icon: PathBuf, tray_menu: Option<Menu>) -> Self {
    let path = icon.parent().expect("Invalid icon");

    let app_indicator: Box<dyn AppIndicator> = if probe_libindicator() {
      Box::new(GtkAppIndicator::with_path(
        "tao application",
        &icon.to_string_lossy(),
        &path.to_string_lossy(),
      ))
    } else if probe_libayatana() {
      Box::new(AyatanaAppIndicator::with_path(
        "tao application",
        &icon.to_string_lossy(),
        &path.to_string_lossy(),
      ))
    } else {
      panic!("libappindicator or libayatana-appindicator is required");
    };

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
  app_indicator: Box<dyn AppIndicator>,
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
