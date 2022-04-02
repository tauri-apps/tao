// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{
  error::OsError, event_loop::EventLoopWindowTarget, system_tray::SystemTray as RootSystemTray,
};

use glib::Sender;
use std::fs::File;
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
  path: PathBuf,
}

impl SystemTrayBuilder {
  #[inline]
  pub fn new(icon: Vec<u8>, tray_menu: Option<Menu>) -> Self {
    let tempfile_path = temp_png_file().expect("Failed to create a temp file for icon");
    if let Ok(mut file) = std::fs::File::create(&tempfile_path) {
      use std::io::Write;
      let _ = file.write_all(icon.as_slice());
    }

    let parent = tempfile_path
      .parent()
      .expect("Failed to get parent of tempfile");
    let mut app_indicator = AppIndicator::new("tao application", "");
    app_indicator.set_icon_theme_path(&parent.to_str().expect("Failed to convert PathBuf to str"));
    app_indicator.set_icon_full(
      &tempfile_path
        .to_str()
        .expect("Failed to convert PathBuf to str"),
      "icon",
    );

    Self {
      tray_menu,
      app_indicator,
      path: tempfile_path,
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
  pub fn set_icon(&mut self, icon: Vec<u8>) {
    let tempfile_path = temp_png_file().expect("Failed to create a temp file for icon");

    if let Ok(mut file) = std::fs::File::create(&tempfile_path) {
      use std::io::Write;
      let _ = file.write_all(icon.as_slice());
    }

    let parent = tempfile_path
      .parent()
      .expect("Failed to get parent of tempfile");
    let mut app_indicator = AppIndicator::new("tao application", "");
    app_indicator.set_icon_theme_path(&parent.to_str().expect("Failed to convert PathBuf to str"));
    app_indicator.set_icon_full(
      &tempfile_path
        .to_str()
        .expect("Failed to convert PathBuf to str"),
      "icon",
    );
    self.path = tempfile_path;
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

fn temp_png_file() -> std::io::Result<PathBuf> {
  let mut path = std::env::temp_dir();
  path.push("tao");
  std::fs::create_dir_all(&path)?;
  path.push(format!("tray-icon-{}.png", uuid::Uuid::new_v4()));
  File::create(path.clone())?;
  Ok(path)
}
