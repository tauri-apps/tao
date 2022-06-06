// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{
  error::OsError,
  event_loop::EventLoopWindowTarget,
  system_tray::{Icon, SystemTray as RootSystemTray},
};

use glib::Sender;
use std::fs::File;
use std::path::PathBuf;

use gtk::{prelude::WidgetExt, AccelGroup};
use libappindicator::{AppIndicator, AppIndicatorStatus};

use super::{menu::Menu, window::WindowRequest, WindowId};

pub struct SystemTrayBuilder {
  tray_menu: Option<Menu>,
  app_indicator: AppIndicator,
  path: PathBuf,
}

impl SystemTrayBuilder {
  #[inline]
  pub fn new(icon: Icon, tray_menu: Option<Menu>) -> Self {
    let (parent_path, icon_path) =
      temp_icon_path().expect("Failed to create a temp folder for icon");
    icon.write_to_png(&icon_path);

    let mut app_indicator = AppIndicator::new("tao application", "");
    app_indicator.set_icon_theme_path(&parent_path.to_string_lossy());
    app_indicator.set_icon_full(&icon_path.to_string_lossy(), "icon");

    Self {
      tray_menu,
      app_indicator,
      path: icon_path,
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
      path: self.path,
    }))
  }
}

pub struct SystemTray {
  app_indicator: AppIndicator,
  sender: Sender<(WindowId, WindowRequest)>,
  path: PathBuf,
}

impl SystemTray {
  pub fn set_icon(&mut self, icon: Icon) {
    let (parent_path, icon_path) =
      temp_icon_path().expect("Failed to create a temp folder for icon");
    icon.write_to_png(&icon_path);

    self
      .app_indicator
      .set_icon_theme_path(&parent_path.to_string_lossy());
    self
      .app_indicator
      .set_icon_full(&icon_path.to_string_lossy(), "icon");
    self.path = icon_path;
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

impl Drop for SystemTray {
  fn drop(&mut self) {
    let _ = std::fs::remove_file(self.path.clone());
  }
}

fn temp_icon_path() -> std::io::Result<(PathBuf, PathBuf)> {
  let mut parent_path = std::env::temp_dir();
  parent_path.push("tao");
  std::fs::create_dir_all(&parent_path)?;
  let mut icon_path = parent_path.clone();
  icon_path.push(format!("tray-icon-{}.png", uuid::Uuid::new_v4()));
  Ok(parent_path, icon_path)
}
