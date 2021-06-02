use crate::{
  error::OsError,
  event_loop::EventLoopWindowTarget,
  menu::ContextMenu,
  platform_impl::{
    SystemTray as SystemTrayPlatform, SystemTrayBuilder as SystemTrayBuilderPlatform,
  },
};
/// Object that allows you to build SystemTray.
pub struct SystemTrayBuilder(SystemTrayBuilderPlatform);

#[cfg(target_os = "linux")]
use std::path::PathBuf;

impl SystemTrayBuilder {
  /// Creates a new SystemTray for platforms where this is appropriate.
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[cfg(not(target_os = "linux"))]
  pub fn new(icon: Vec<u8>, tray_menu: Option<ContextMenu>) -> Self {
    Self(SystemTrayBuilderPlatform::new(
      icon,
      tray_menu.map(|m| m.0.menu_platform),
    ))
  }

  /// Creates a new SystemTray for platforms where this is appropriate.
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[cfg(target_os = "linux")]
  pub fn new(icon: PathBuf, tray_menu: Option<ContextMenu>) -> Self {
    Self(SystemTrayBuilderPlatform::new(
      icon,
      tray_menu.map(|m| m.0.menu_platform),
    ))
  }

  /// Builds the SystemTray.
  ///
  /// Possible causes of error include denied permission, incompatible system, and lack of memory.
  pub fn build<T: 'static>(
    self,
    _window_target: &EventLoopWindowTarget<T>,
  ) -> Result<SystemTray, OsError> {
    self.0.build(&_window_target)
  }
}

/// Represents a System Tray icon.
pub struct SystemTray(pub SystemTrayPlatform);

impl SystemTray {
  /// Set new tray icon.
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[cfg(not(target_os = "linux"))]
  pub fn set_icon(&mut self, icon: Vec<u8>) {
    self.0.set_icon(icon)
  }

  /// Set new tray icon.
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[cfg(target_os = "linux")]
  pub fn set_icon(&mut self, icon: PathBuf) {
    self.0.set_icon(icon)
  }
}
