// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{
  accelerator::Accelerator,
  error::OsError,
  event_loop::EventLoopWindowTarget,
  keyboard::{KeyCode, ModifiersState},
  system_tray::SystemTray as RootSystemTray,
};

use glib::Sender;
use std::path::PathBuf;

use super::{menu::Menu, window::WindowRequest, WindowId};

pub struct SystemTrayBuilder {
  tray_menu: Option<Menu>,
  icon: PathBuf,
}

impl SystemTrayBuilder {
  #[inline]
  pub fn new(icon: PathBuf, tray_menu: Option<Menu>) -> Self {
    Self { tray_menu, icon }
  }

  #[inline]
  pub fn build<T: 'static>(
    self,
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<RootSystemTray, OsError> {
    let sender = window_target.p.window_requests_tx.clone();
    let tray = match &self.tray_menu {
      Some(m) => KsniTray::new_with_menu("tao application", &self.icon, &m, sender),
      None => KsniTray::new("tao application", &self.icon, sender),
    };

    Ok(RootSystemTray(SystemTray::new(tray)))
  }
}

pub struct SystemTray {
  tray_handle: ksni::Handle<KsniTray>,
}

impl SystemTray {
  pub fn new(tray: KsniTray) -> Self {
    let tray_service = ksni::TrayService::new(tray);
    let tray_handle = tray_service.handle();
    tray_service.spawn();

    Self {
      tray_handle: tray_handle,
    }
  }

  pub fn set_icon(&mut self, icon: PathBuf) {
    self.tray_handle.update(|tray: &mut KsniTray| {
      tray.set_icon(&icon);
    });
  }

  pub fn set_menu(&mut self, tray_menu: &Menu) {
    self.tray_handle.update(|tray: &mut KsniTray| {
      tray.set_menu(tray_menu.clone());
    });
  }
}

/// Type alias for ksni menu.
pub(crate) type KsniMenu = Vec<ksni::MenuItem<KsniTray>>;
pub(crate) type KsniMenuItem = ksni::MenuItem<KsniTray>;

/// Holds all properties and signals of the tray and manages the communcation via DBus.
pub struct KsniTray {
  title: String,
  icon_name: String,
  icon_theme_path: String,
  status: ksni::Status,
  menu: Option<Menu>,
  sender: Sender<(WindowId, WindowRequest)>,
}

unsafe impl Send for KsniTray {}

impl KsniTray {
  /// Initializes a new instance.
  ///
  /// # Arguments
  ///
  /// * `title` -  The instance title.
  /// * `icon` -  Absolute file path to the icon that will be visible in tray.
  /// * `sender` -  Information about the window.
  ///
  /// Initial status is set to `ksni::Status::Active`
  pub fn new(title: &str, icon: &PathBuf, sender: Sender<(WindowId, WindowRequest)>) -> Self {
    let (icon_name, icon_theme_path) = Self::split_icon(&icon);

    Self {
      title: title.to_string(),
      icon_name,
      icon_theme_path,
      menu: None,
      status: ksni::Status::Active,
      sender,
    }
  }

  /// Initializes a new instance including a menu.
  ///
  /// # Arguments
  ///
  /// * `title` -  The instance title.
  /// * `icon` -  Absolute file path to the icon that will be visible in tray.
  /// * `menu` -  The menu belonging to the tray icon.
  /// * `sender` -  Information about the window.
  ///
  /// Initial status is set to `ksni::Status::Active`
  pub fn new_with_menu(
    title: &str,
    icon: &PathBuf,
    menu: &Menu,
    sender: Sender<(WindowId, WindowRequest)>,
  ) -> Self {
    let (icon_name, icon_theme_path) = Self::split_icon(&icon);

    Self {
      title: title.to_string(),
      icon_name,
      icon_theme_path,
      menu: Some(menu.clone()),
      status: ksni::Status::Active,
      sender,
    }
  }

  /// Updates the icon.
  pub fn set_icon(&mut self, icon: &PathBuf) {
    let (icon_name, icon_theme_path) = Self::split_icon(&icon);
    self.icon_name = icon_name;
    self.icon_theme_path = icon_theme_path;
  }

  /// Updates the menu.
  pub fn set_menu(&mut self, menu: Menu) {
    self.menu = Some(menu);
  }

  /// Splits the given icon path into the folder and the filename only, as it
  /// is required by ksni.
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

  fn menu(&self) -> KsniMenu {
    if let Some(m) = &self.menu {
      m.to_ksnimenu(&self.sender, WindowId::dummy())
    } else {
      vec![]
    }
  }
}

/// Small wrapper struct that is used to convert [`Accelerator`]
/// to the shortcut of [`ksni::MenuItem`].
pub struct KsniAccelerator {
  modifier: String,
  key: String,
}

impl KsniAccelerator {
  fn to_modifier(accel_modifier: ModifiersState) -> String {
    match accel_modifier {
      ModifiersState::SHIFT => "Shift".into(),
      ModifiersState::CONTROL => "Control".into(),
      ModifiersState::ALT => "Alt".into(),
      ModifiersState::SUPER => "Super".into(),
      _ => {
        dbg!("Cannot map modifier {:?}", accel_modifier);
        "".into()
      }
    }
  }

  fn to_key(accel_key: KeyCode) -> String {
    match &accel_key {
      KeyCode::KeyA => 'A'.into(),
      KeyCode::KeyB => 'B'.into(),
      KeyCode::KeyC => 'C'.into(),
      KeyCode::KeyD => 'D'.into(),
      KeyCode::KeyE => 'E'.into(),
      KeyCode::KeyF => 'F'.into(),
      KeyCode::KeyG => 'G'.into(),
      KeyCode::KeyH => 'H'.into(),
      KeyCode::KeyI => 'I'.into(),
      KeyCode::KeyJ => 'J'.into(),
      KeyCode::KeyK => 'K'.into(),
      KeyCode::KeyL => 'L'.into(),
      KeyCode::KeyM => 'M'.into(),
      KeyCode::KeyN => 'N'.into(),
      KeyCode::KeyO => 'O'.into(),
      KeyCode::KeyP => 'P'.into(),
      KeyCode::KeyQ => 'Q'.into(),
      KeyCode::KeyR => 'R'.into(),
      KeyCode::KeyS => 'S'.into(),
      KeyCode::KeyT => 'T'.into(),
      KeyCode::KeyU => 'U'.into(),
      KeyCode::KeyV => 'V'.into(),
      KeyCode::KeyW => 'W'.into(),
      KeyCode::KeyX => 'X'.into(),
      KeyCode::KeyY => 'Y'.into(),
      KeyCode::KeyZ => 'Z'.into(),
      KeyCode::Digit0 => '0'.into(),
      KeyCode::Digit1 => '1'.into(),
      KeyCode::Digit2 => '2'.into(),
      KeyCode::Digit3 => '3'.into(),
      KeyCode::Digit4 => '4'.into(),
      KeyCode::Digit5 => '5'.into(),
      KeyCode::Digit6 => '6'.into(),
      KeyCode::Digit7 => '7'.into(),
      KeyCode::Digit8 => '8'.into(),
      KeyCode::Digit9 => '9'.into(),
      _ => {
        dbg!("Cannot map key {:?}", accel_key);
        "".into()
      }
    }
  }

  /// Returns the accelerator combination that can be put to the shortcut property
  /// of [`ksni::MenuItem`]
  pub fn into_vec(self) -> Vec<String> {
    vec![self.modifier, self.key]
  }
}

impl From<&Accelerator> for KsniAccelerator {
  fn from(accelerator: &Accelerator) -> Self {
    Self {
      modifier: Self::to_modifier(accelerator.mods),
      key: Self::to_key(accelerator.key),
    }
  }
}
