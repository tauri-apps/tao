// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{error::OsError, platform_impl::EventLoopWindowTarget};

use std::path::PathBuf;
use std::{borrow::Cow, fmt};

pub struct Indicator(libappindicator::AppIndicator);

impl fmt::Debug for Indicator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Indicator").finish()
  }
}

impl std::ops::Deref for Indicator {
  type Target = libappindicator::AppIndicator;

  fn deref(&self) -> &libappindicator::AppIndicator {
    &self.0
  }
}

impl std::ops::DerefMut for Indicator {
  fn deref_mut(&mut self) -> &mut libappindicator::AppIndicator {
    &mut self.0
  }
}

#[derive(Debug)]
pub struct SystemTray {
  indicator: Option<Indicator>,
}

impl SystemTray {
  pub fn new() -> Self {
    SystemTray { indicator: None }
  }

  #[cfg(feature = "tray")]
  pub(crate) fn initialize<T>(
    &mut self,
    window_target: &EventLoopWindowTarget<T>,
    icon: &PathBuf,
    items: &Vec<crate::menu::MenuItem>,
  ) -> Result<(), OsError> {
    use crate::menu::MenuItem;
    use gtk::prelude::*;
    use libappindicator::{AppIndicator, AppIndicatorStatus};

    use super::window::{WindowId, WindowRequest};

    let (stem, path) = get_system_tray_icon_stem_and_path(icon)?;
    let mut indicator = AppIndicator::with_path("tao application", &stem, &path);
    indicator.set_status(AppIndicatorStatus::Active);
    let mut m = gtk::Menu::new();

    for i in items.iter() {
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
    self.indicator = Some(Indicator(indicator));
    m.show_all();
    Ok(())
  }

  #[cfg(feature = "tray")]
  pub(crate) fn set_icon(&mut self, icon: &PathBuf) -> Result<(), OsError> {
    let (stem, path) = get_system_tray_icon_stem_and_path(icon)?;
    match &mut self.indicator {
      Some(indicator) => {
        indicator.set_icon_theme_path(&path);
        indicator.set_icon(&stem);
        Ok(())
      }
      None => Err(OsError::new(
        96,
        "system tray must be initialized first",
        super::OsError,
      )),
    }
  }

  #[cfg(not(feature = "tray"))]
  pub(crate) fn initialize<T>(
    _window_target: &EventLoopWindowTarget<T>,
    _icon: &PathBuf,
    _items: &Vec<crate::menu::MenuItem>,
  ) -> Result<(), OsError> {
    Ok(())
  }

  #[cfg(not(feature = "tray"))]
  pub(crate) fn set_icon(&self, _icon: PathBuf) -> Result<(), OsError> {
    Ok(())
  }
}

fn get_system_tray_icon_stem_and_path(
  icon: &PathBuf,
) -> Result<(Cow<'_, str>, Cow<'_, str>), OsError> {
  let stem = match icon.file_stem() {
    Some(name) => name.to_string_lossy(),
    None => return Err(OsError::new(123, "system tray icon", super::OsError)),
  };
  let path = match icon.parent() {
    Some(name) => name.to_string_lossy(),
    None => return Err(OsError::new(127, "system tray icon", super::OsError)),
  };
  Ok((stem, path))
}
