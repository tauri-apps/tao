// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{error::OsError, event_loop::EventLoopWindowTarget};

use std::path::PathBuf;

use gtk::AccelGroup;
use gtk::WidgetExt;
use libappindicator::{AppIndicator, AppIndicatorStatus};

use super::menu::Menu;
use super::WindowId;

pub struct SystemTrayBuilder {
  pub(crate) system_tray: SystemTray,
}

impl SystemTrayBuilder {
  /// Creates a new SystemTray for platforms where this is appropriate.
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[inline]
  pub fn new(icon: PathBuf, tray_menu: Menu) -> Self {
    let path = icon.parent().expect("Invalid icon");
    let app_indicator = AppIndicator::with_path(
      "tao application",
      &icon.to_string_lossy(),
      &path.to_string_lossy(),
    );
    Self {
      system_tray: SystemTray {
        icon,
        tray_menu,
        app_indicator,
      },
    }
  }

  /// Builds the system tray.
  ///
  /// Possible causes of error include denied permission, incompatible system, and lack of memory.
  #[inline]
  pub fn build<T: 'static>(
    mut self,
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<SystemTray, OsError> {
    let tx_ = window_target.p.window_requests_tx.clone();

    let tray_menu = self.system_tray.tray_menu.clone();
    let menu = &mut tray_menu.into_gtkmenu(&tx_, &AccelGroup::new(), WindowId::dummy());

    self
      .system_tray
      .app_indicator
      .set_status(AppIndicatorStatus::Active);
    self.system_tray.app_indicator.set_menu(menu);
    menu.show_all();

    Ok(self.system_tray)
  }
}

pub struct SystemTray {
  icon: PathBuf,
  tray_menu: Menu,
  app_indicator: AppIndicator,
}

impl SystemTray {
  pub fn set_icon(&mut self, icon: Vec<u8>) {
    // todo allow swapping icon
  }
}
