// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{
  error::OsError, event_loop::EventLoopWindowTarget, system_tray::SystemTray as RootSystemTray,
};

use glib::Sender;
use std::path::PathBuf;

use gtk::{AccelGroup, WidgetExt};

use super::{menu::Menu, window::WindowRequest, WindowId};

pub struct SystemTrayBuilder {
  tray_menu: Option<Menu>,
  tray: KsniTray,
}

impl SystemTrayBuilder {
  #[inline]
  pub fn new(icon: PathBuf, tray_menu: Option<Menu>) -> Self {
    let tray = KsniTray::new("tao application", &icon);

    Self { tray_menu, tray }
  }

  #[inline]
  pub fn build<T: 'static>(
    mut self,
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<RootSystemTray, OsError> {
    let sender = window_target.p.window_requests_tx.clone();

    Ok(RootSystemTray(SystemTray::new(self.tray, sender)))
  }
}

pub struct SystemTray {
  tray_handle: ksni::Handle<KsniTray>,
  sender: Sender<(WindowId, WindowRequest)>,
}

impl SystemTray {
  pub fn new(tray: KsniTray, sender: Sender<(WindowId, WindowRequest)>) -> Self {
    let tray_service = ksni::TrayService::new(tray);
    let tray_handle = tray_service.handle();
    tray_service.spawn();

    Self {
      tray_handle: tray_handle,
      sender,
    }
  }

  pub fn set_icon(&mut self, icon: PathBuf) {
    self.tray_handle.update(|tray: &mut KsniTray| {
      tray.set_icon(&icon);
    });
  }

  pub fn set_menu(&mut self, tray_menu: &Menu) {
    //let mut menu =
    //  tray_menu
    //    .clone()
    //    .into_gtkmenu(&self.sender, &AccelGroup::new(), WindowId::dummy());

    //self.app_indicator.set_menu(&mut menu);
    //menu.show_all();
  }
}

/// Holds all properties and signals of the tray and manages the communcation via DBus.
pub struct KsniTray {
  title: String,
  icon_name: String,
  icon_theme_path: String,
  status: ksni::Status,
}

impl KsniTray {
  /// Initializes a new instance.
  ///
  /// # Arguments
  ///
  /// * `title` -  The instance title.
  /// * `icon` -  Absolute file path to the icon that will be visible in tray.
  ///
  /// Initial status is set to `ksni::Status::Active`
  pub fn new(title: &str, icon: &PathBuf) -> Self {
    let (icon_name, icon_theme_path) = Self::split_icon(&icon);

    Self {
      title: title.to_string(),
      icon_name,
      icon_theme_path,
      status: ksni::Status::Active,
    }
  }

  /// Updates the icon.
  pub fn set_icon(&mut self, icon: &PathBuf) {
    let (icon_name, icon_theme_path) = Self::split_icon(&icon);
    self.icon_name = icon_name;
    self.icon_theme_path = icon_theme_path;
  }

  fn split_icon(icon: &PathBuf) -> (String, String) {
    (
      icon
        .file_stem()
        .expect("Invalid icon name!")
        .to_string_lossy()
        .into(),
      icon
        .parent()
        .expect("Invalid icon theme path!")
        .to_string_lossy()
        .into(),
    )
  }
}

impl ksni::Tray for KsniTray {
  fn title(&self) -> String {
    self.title.clone()
  }

  fn icon_name(&self) -> String {
    self.icon_name.clone()
  }

  fn icon_theme_path(&self) -> String {
    self.icon_theme_path.clone()
  }

  fn status(&self) -> ksni::Status {
    self.status
  }
}
