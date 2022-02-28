// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{
  error::OsError,
  event_loop::EventLoopWindowTarget,
  system_tray::{Icon, SystemTray as RootSystemTray},
};

use glib::Sender;
use std::io::Write;
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
  pub fn new(icon: Icon, tray_menu: Option<Menu>) -> Self {
    let mut tempfile =
      tempfile::NamedTempFile::new().expect("Failed to create a temp file for icon");
    tempfile
      .write(icon.inner.raw.as_slice())
      .expect("Failed to write image to disk");

    let path = tempfile.path().to_path_buf();
    let app_indicator = AppIndicator::new(
      "tao application",
      &path.to_str().expect("Failed to convert PathBuf to str"),
    );

    Self {
      tray_menu,
      app_indicator,
      path,
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
    let mut tempfile =
      tempfile::NamedTempFile::new().expect("Failed to create a temp file for icon");
    tempfile
      .write(icon.inner.raw.as_slice())
      .expect("Failed to write image to disk");

    let path = tempfile.path().to_path_buf();
    self
      .app_indicator
      .set_icon(&path.to_str().expect("Failed to convert PathBuf to str"));
    self.path = path;
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
