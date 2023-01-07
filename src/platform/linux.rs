// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(target_os = "linux")]

#[cfg(feature = "tray")]
use crate::system_tray::SystemTrayBuilder;
#[cfg(feature = "tray")]
use std::path::Path;

#[cfg(feature = "tray")]
pub trait SystemTrayBuilderExtLinux {
  /// Sets a custom temp icon dir to store generated icon files.
  fn with_temp_icon_dir<P: AsRef<Path>>(self, p: P) -> Self;
}

#[cfg(feature = "tray")]
impl SystemTrayBuilderExtLinux for SystemTrayBuilder {
  fn with_temp_icon_dir<P: AsRef<Path>>(mut self, p: P) -> Self {
    self.platform_tray_builder.temp_icon_dir = Some(p.as_ref().to_path_buf());
    self
  }
}
