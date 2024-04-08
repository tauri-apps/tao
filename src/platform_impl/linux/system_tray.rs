// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{
  error::OsError,
  event_loop::EventLoopWindowTarget,
  system_tray::{Icon, SystemTray as RootSystemTray},
  TrayId,
};

use glib::Sender;
use std::path::PathBuf;

use gtk::{prelude::WidgetExt, AccelGroup};
use libappindicator::{AppIndicator, AppIndicatorStatus};

use super::{menu::Menu, window::WindowRequest, WindowId};

pub struct SystemTrayBuilder {
  pub(crate) temp_icon_dir: Option<PathBuf>,
  tray_menu: Option<Menu>,
  icon: Icon,
}

impl SystemTrayBuilder {
  #[inline]
  pub fn new(icon: Icon, tray_menu: Option<Menu>) -> Self {
    Self {
      temp_icon_dir: None,
      tray_menu,
      icon,
    }
  }

  #[inline]
  pub fn build<T: 'static>(
    self,
    window_target: &EventLoopWindowTarget<T>,
    _id: TrayId,
    _tooltip: Option<String>,
  ) -> Result<RootSystemTray, OsError> {
    let mut app_indicator = AppIndicator::new("tao application", "");

    let (parent_path, icon_path) = temp_icon_path(self.temp_icon_dir.as_ref()).map_err(|e| os_error!(e.into()))?;

    self.icon.inner.write_to_png(&icon_path)?;

    app_indicator.set_icon_theme_path(&parent_path.to_string_lossy());
    app_indicator.set_icon_full(&icon_path.to_string_lossy(), "icon");

    let sender = window_target.p.window_requests_tx.clone();

    if let Some(tray_menu) = self.tray_menu.clone() {
      let menu = &mut tray_menu.into_gtkmenu(&sender, &AccelGroup::new(), WindowId::dummy());

      app_indicator.set_menu(menu);
      menu.show_all();
    }

    app_indicator.set_status(AppIndicatorStatus::Active);

    Ok(RootSystemTray(SystemTray {
      temp_icon_dir: self.temp_icon_dir,
      app_indicator,
      sender,
      path: icon_path,
    }))
  }
}

pub struct SystemTray {
  temp_icon_dir: Option<PathBuf>,
  app_indicator: AppIndicator,
  sender: Sender<(WindowId, WindowRequest)>,
  path: PathBuf,
}

impl SystemTray {
  pub fn set_icon(&mut self, icon: Icon) {
    if let Ok((parent_path, icon_path)) = temp_icon_path(self.temp_icon_dir.as_ref()) {
      let _ = icon.inner.write_to_png(&icon_path);
      self
        .app_indicator
        .set_icon_theme_path(&parent_path.to_string_lossy());
      self
        .app_indicator
        .set_icon_full(&icon_path.to_string_lossy(), "icon");
      self.path = icon_path;
    }
  }

  pub fn set_tooltip(&self, _tooltip: &str) {}

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
    self.app_indicator.set_status(AppIndicatorStatus::Passive);
    let _ = std::fs::remove_file(self.path.clone());
  }
}

/// Generates an icon path in one of the following dirs:
/// 1. If `temp_icon_dir` is `Some` use that.
/// 2. `$XDG_RUNTIME_DIR/tao`
/// 3. `/tmp/tao`
fn temp_icon_path(temp_icon_dir: Option<&PathBuf>) -> std::io::Result<(PathBuf, PathBuf)> {
  let parent_path = match temp_icon_dir.as_ref() {
    Some(path) => path.to_path_buf(),
    None => dirs_next::runtime_dir()
      .unwrap_or_else(|| std::env::temp_dir())
      .join("tao"),
  };

  std::fs::create_dir_all(&parent_path)?;
  let icon_path = parent_path.join(format!("tray-icon-{}.png", uuid::Uuid::new_v4()));
  Ok((parent_path, icon_path))
}

#[test]
fn temp_icon_path_preference_order() {
  let runtime_dir = option_env!("XDG_RUNTIME_DIR");
  let override_dir = PathBuf::from("/tmp/tao-tests");

  let (dir1, _file1) = temp_icon_path(Some(&override_dir)).unwrap();
  let (dir2, _file1) = temp_icon_path(None).unwrap();
  std::env::remove_var("XDG_RUNTIME_DIR");
  let (dir3, _file2) = temp_icon_path(None).unwrap();

  assert_eq!(dir1, override_dir);
  if let Some(runtime_dir) = runtime_dir {
    std::env::set_var("XDG_RUNTIME_DIR", runtime_dir);
    assert_eq!(dir2, PathBuf::from(format!("{}/tao", runtime_dir)));
  }

  assert_eq!(dir3, PathBuf::from("/tmp/tao"));
}
