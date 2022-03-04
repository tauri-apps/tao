// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(target_os = "macos")]

use std::os::raw::c_void;

use crate::{
  dpi::LogicalSize,
  event_loop::{EventLoop, EventLoopWindowTarget},
  menu::CustomMenuItem,
  monitor::MonitorHandle,
  platform_impl::{get_aux_state_mut, Parent},
  window::{Window, WindowBuilder},
};

#[cfg(feature = "tray")]
use crate::system_tray::{SystemTray, SystemTrayBuilder};

use cocoa::{appkit, base::id};

/// Additional methods on `Window` that are specific to MacOS.
pub trait WindowExtMacOS {
  /// Returns a pointer to the cocoa `NSWindow` that is used by this window.
  ///
  /// The pointer will become invalid when the `Window` is destroyed.
  fn ns_window(&self) -> *mut c_void;

  /// Returns a pointer to the cocoa `NSView` that is used by this window.
  ///
  /// The pointer will become invalid when the `Window` is destroyed.
  fn ns_view(&self) -> *mut c_void;

  /// Returns whether or not the window is in simple fullscreen mode.
  fn simple_fullscreen(&self) -> bool;

  /// Toggles a fullscreen mode that doesn't require a new macOS space.
  /// Returns a boolean indicating whether the transition was successful (this
  /// won't work if the window was already in the native fullscreen).
  ///
  /// This is how fullscreen used to work on macOS in versions before Lion.
  /// And allows the user to have a fullscreen window without using another
  /// space or taking control over the entire monitor.
  fn set_simple_fullscreen(&self, fullscreen: bool) -> bool;

  /// Returns whether or not the window has shadow.
  fn has_shadow(&self) -> bool;

  /// Sets whether or not the window has shadow.
  fn set_has_shadow(&self, has_shadow: bool);
}

impl WindowExtMacOS for Window {
  #[inline]
  fn ns_window(&self) -> *mut c_void {
    self.window.ns_window()
  }

  #[inline]
  fn ns_view(&self) -> *mut c_void {
    self.window.ns_view()
  }

  #[inline]
  fn simple_fullscreen(&self) -> bool {
    self.window.simple_fullscreen()
  }

  #[inline]
  fn set_simple_fullscreen(&self, fullscreen: bool) -> bool {
    self.window.set_simple_fullscreen(fullscreen)
  }

  #[inline]
  fn has_shadow(&self) -> bool {
    self.window.has_shadow()
  }

  #[inline]
  fn set_has_shadow(&self, has_shadow: bool) {
    self.window.set_has_shadow(has_shadow)
  }
}

/// Corresponds to `NSApplicationActivationPolicy`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivationPolicy {
  /// Corresponds to `NSApplicationActivationPolicyRegular`.
  Regular,
  /// Corresponds to `NSApplicationActivationPolicyAccessory`.
  Accessory,
  /// Corresponds to `NSApplicationActivationPolicyProhibited`.
  Prohibited,
}

impl Default for ActivationPolicy {
  fn default() -> Self {
    ActivationPolicy::Regular
  }
}

pub trait CustomMenuItemExtMacOS {
  fn set_native_image(&mut self, native_image: NativeImage);
}

impl CustomMenuItemExtMacOS for CustomMenuItem {
  fn set_native_image(&mut self, native_image: NativeImage) {
    self.0.set_native_image(native_image)
  }
}

/// Named images, defined by the system or you, for use in your app.
#[non_exhaustive]
pub enum NativeImage {
  /// An add item template image.
  Add,
  /// Advanced preferences toolbar icon for the preferences window.
  Advanced,
  /// A Bluetooth template image.
  Bluetooth,
  /// Bookmarks image suitable for a template.
  Bookmarks,
  /// A caution image.
  Caution,
  /// A color panel toolbar icon.
  ColorPanel,
  /// A column view mode template image.
  ColumnView,
  /// A computer icon.
  Computer,
  /// An enter full-screen mode template image.
  EnterFullScreen,
  /// Permissions for all users.
  Everyone,
  /// An exit full-screen mode template image.
  ExitFullScreen,
  /// A cover flow view mode template image.
  FlowView,
  /// A folder image.
  Folder,
  /// A burnable folder icon.
  FolderBurnable,
  /// A smart folder icon.
  FolderSmart,
  /// A link template image.
  FollowLinkFreestanding,
  /// A font panel toolbar icon.
  FontPanel,
  /// A `go back` template image.
  GoLeft,
  /// A `go forward` template image.
  GoRight,
  /// Home image suitable for a template.
  Home,
  /// An iChat Theater template image.
  IChatTheater,
  /// An icon view mode template image.
  IconView,
  /// An information toolbar icon.
  Info,
  /// A template image used to denote invalid data.
  InvalidDataFreestanding,
  /// A generic left-facing triangle template image.
  LeftFacingTriangle,
  /// A list view mode template image.
  ListView,
  /// A locked padlock template image.
  LockLocked,
  /// An unlocked padlock template image.
  LockUnlocked,
  /// A horizontal dash, for use in menus.
  MenuMixedState,
  /// A check mark template image, for use in menus.
  MenuOnState,
  /// A MobileMe icon.
  MobileMe,
  /// A drag image for multiple items.
  MultipleDocuments,
  /// A network icon.
  Network,
  /// A path button template image.
  Path,
  /// General preferences toolbar icon for the preferences window.
  PreferencesGeneral,
  /// A Quick Look template image.
  QuickLook,
  /// A refresh template image.
  RefreshFreestanding,
  /// A refresh template image.
  Refresh,
  /// A remove item template image.
  Remove,
  /// A reveal contents template image.
  RevealFreestanding,
  /// A generic right-facing triangle template image.
  RightFacingTriangle,
  /// A share view template image.
  Share,
  /// A slideshow template image.
  Slideshow,
  /// A badge for a `smart` item.
  SmartBadge,
  /// Small green indicator, similar to iChat’s available image.
  StatusAvailable,
  /// Small clear indicator.
  StatusNone,
  /// Small yellow indicator, similar to iChat’s idle image.
  StatusPartiallyAvailable,
  /// Small red indicator, similar to iChat’s unavailable image.
  StatusUnavailable,
  /// A stop progress template image.
  StopProgressFreestanding,
  /// A stop progress button template image.
  StopProgress,

  // todo add TouchBar icons
  // https://developer.apple.com/documentation/appkit/nsimagenameactiontemplate
  /// An image of the empty trash can.
  TrashEmpty,
  /// An image of the full trash can.
  TrashFull,
  /// Permissions for a single user.
  User,
  /// User account toolbar icon for the preferences window.
  UserAccounts,
  /// Permissions for a group of users.
  UserGroup,
  /// Permissions for guests.
  UserGuest,
}

impl NativeImage {
  pub(crate) unsafe fn get_ns_image(self) -> id {
    match self {
      NativeImage::Add => appkit::NSImageNameAddTemplate,
      NativeImage::StatusAvailable => appkit::NSImageNameStatusAvailable,
      NativeImage::StatusUnavailable => appkit::NSImageNameStatusUnavailable,
      NativeImage::StatusPartiallyAvailable => appkit::NSImageNameStatusPartiallyAvailable,
      NativeImage::Advanced => appkit::NSImageNameAdvanced,
      NativeImage::Bluetooth => appkit::NSImageNameBluetoothTemplate,
      NativeImage::Bookmarks => appkit::NSImageNameBookmarksTemplate,
      NativeImage::Caution => appkit::NSImageNameCaution,
      NativeImage::ColorPanel => appkit::NSImageNameColorPanel,
      NativeImage::ColumnView => appkit::NSImageNameColumnViewTemplate,
      NativeImage::Computer => appkit::NSImageNameComputer,
      NativeImage::EnterFullScreen => appkit::NSImageNameEnterFullScreenTemplate,
      NativeImage::Everyone => appkit::NSImageNameEveryone,
      NativeImage::ExitFullScreen => appkit::NSImageNameExitFullScreenTemplate,
      NativeImage::FlowView => appkit::NSImageNameFlowViewTemplate,
      NativeImage::Folder => appkit::NSImageNameFolder,
      NativeImage::FolderBurnable => appkit::NSImageNameFolderBurnable,
      NativeImage::FolderSmart => appkit::NSImageNameFolderSmart,
      NativeImage::FollowLinkFreestanding => appkit::NSImageNameFollowLinkFreestandingTemplate,
      NativeImage::FontPanel => appkit::NSImageNameFontPanel,
      NativeImage::GoLeft => appkit::NSImageNameGoLeftTemplate,
      NativeImage::GoRight => appkit::NSImageNameGoRightTemplate,
      NativeImage::Home => appkit::NSImageNameHomeTemplate,
      NativeImage::IChatTheater => appkit::NSImageNameIChatTheaterTemplate,
      NativeImage::IconView => appkit::NSImageNameIconViewTemplate,
      NativeImage::Info => appkit::NSImageNameInfo,
      NativeImage::InvalidDataFreestanding => appkit::NSImageNameInvalidDataFreestandingTemplate,
      NativeImage::LeftFacingTriangle => appkit::NSImageNameLeftFacingTriangleTemplate,
      NativeImage::ListView => appkit::NSImageNameListViewTemplate,
      NativeImage::LockLocked => appkit::NSImageNameLockLockedTemplate,
      NativeImage::LockUnlocked => appkit::NSImageNameLockUnlockedTemplate,
      NativeImage::MenuMixedState => appkit::NSImageNameMenuMixedStateTemplate,
      NativeImage::MenuOnState => appkit::NSImageNameMenuOnStateTemplate,
      NativeImage::MobileMe => appkit::NSImageNameMobileMe,
      NativeImage::MultipleDocuments => appkit::NSImageNameMultipleDocuments,
      NativeImage::Network => appkit::NSImageNameNetwork,
      NativeImage::Path => appkit::NSImageNamePathTemplate,
      NativeImage::PreferencesGeneral => appkit::NSImageNamePreferencesGeneral,
      NativeImage::QuickLook => appkit::NSImageNameQuickLookTemplate,
      NativeImage::RefreshFreestanding => appkit::NSImageNameRefreshFreestandingTemplate,
      NativeImage::Refresh => appkit::NSImageNameRefreshTemplate,
      NativeImage::Remove => appkit::NSImageNameRemoveTemplate,
      NativeImage::RevealFreestanding => appkit::NSImageNameRevealFreestandingTemplate,
      NativeImage::RightFacingTriangle => appkit::NSImageNameRightFacingTriangleTemplate,
      NativeImage::Share => appkit::NSImageNameShareTemplate,
      NativeImage::Slideshow => appkit::NSImageNameSlideshowTemplate,
      NativeImage::SmartBadge => appkit::NSImageNameSmartBadgeTemplate,
      NativeImage::StatusNone => appkit::NSImageNameStatusNone,
      NativeImage::StopProgressFreestanding => appkit::NSImageNameStopProgressFreestandingTemplate,
      NativeImage::StopProgress => appkit::NSImageNameStopProgressTemplate,
      NativeImage::TrashEmpty => appkit::NSImageNameTrashEmpty,
      NativeImage::TrashFull => appkit::NSImageNameTrashFull,
      NativeImage::User => appkit::NSImageNameUser,
      NativeImage::UserAccounts => appkit::NSImageNameUserAccounts,
      NativeImage::UserGroup => appkit::NSImageNameUserGroup,
      NativeImage::UserGuest => appkit::NSImageNameUserGuest,
    }
  }
}

/// Additional methods on `WindowBuilder` that are specific to MacOS.
///
/// **Note:** Properties dealing with the titlebar will be overwritten by the `with_decorations` method
/// on the base `WindowBuilder`:
///
///  - `with_titlebar_transparent`
///  - `with_title_hidden`
///  - `with_titlebar_hidden`
///  - `with_titlebar_buttons_hidden`
///  - `with_fullsize_content_view`
pub trait WindowBuilderExtMacOS {
  /// Sets a parent to the window to be created.
  fn with_parent_window(self, parent: *mut c_void) -> WindowBuilder;
  /// Enables click-and-drag behavior for the entire window, not just the titlebar.
  fn with_movable_by_window_background(self, movable_by_window_background: bool) -> WindowBuilder;
  /// Makes the titlebar transparent and allows the content to appear behind it.
  fn with_titlebar_transparent(self, titlebar_transparent: bool) -> WindowBuilder;
  /// Hides the window title.
  fn with_title_hidden(self, title_hidden: bool) -> WindowBuilder;
  /// Hides the window titlebar.
  fn with_titlebar_hidden(self, titlebar_hidden: bool) -> WindowBuilder;
  /// Hides the window titlebar buttons.
  fn with_titlebar_buttons_hidden(self, titlebar_buttons_hidden: bool) -> WindowBuilder;
  /// Makes the window content appear behind the titlebar.
  fn with_fullsize_content_view(self, fullsize_content_view: bool) -> WindowBuilder;
  /// Build window with `resizeIncrements` property. Values must not be 0.
  fn with_resize_increments(self, increments: LogicalSize<f64>) -> WindowBuilder;
  fn with_disallow_hidpi(self, disallow_hidpi: bool) -> WindowBuilder;
  fn with_has_shadow(self, has_shadow: bool) -> WindowBuilder;
}

impl WindowBuilderExtMacOS for WindowBuilder {
  #[inline]
  fn with_parent_window(mut self, parent: *mut c_void) -> WindowBuilder {
    self.platform_specific.parent = Parent::ChildOf(parent);
    self
  }

  #[inline]
  fn with_movable_by_window_background(
    mut self,
    movable_by_window_background: bool,
  ) -> WindowBuilder {
    self.platform_specific.movable_by_window_background = movable_by_window_background;
    self
  }

  #[inline]
  fn with_titlebar_transparent(mut self, titlebar_transparent: bool) -> WindowBuilder {
    self.platform_specific.titlebar_transparent = titlebar_transparent;
    self
  }

  #[inline]
  fn with_titlebar_hidden(mut self, titlebar_hidden: bool) -> WindowBuilder {
    self.platform_specific.titlebar_hidden = titlebar_hidden;
    self
  }

  #[inline]
  fn with_titlebar_buttons_hidden(mut self, titlebar_buttons_hidden: bool) -> WindowBuilder {
    self.platform_specific.titlebar_buttons_hidden = titlebar_buttons_hidden;
    self
  }

  #[inline]
  fn with_title_hidden(mut self, title_hidden: bool) -> WindowBuilder {
    self.platform_specific.title_hidden = title_hidden;
    self
  }

  #[inline]
  fn with_fullsize_content_view(mut self, fullsize_content_view: bool) -> WindowBuilder {
    self.platform_specific.fullsize_content_view = fullsize_content_view;
    self
  }

  #[inline]
  fn with_resize_increments(mut self, increments: LogicalSize<f64>) -> WindowBuilder {
    self.platform_specific.resize_increments = Some(increments);
    self
  }

  #[inline]
  fn with_disallow_hidpi(mut self, disallow_hidpi: bool) -> WindowBuilder {
    self.platform_specific.disallow_hidpi = disallow_hidpi;
    self
  }

  #[inline]
  fn with_has_shadow(mut self, has_shadow: bool) -> WindowBuilder {
    self.platform_specific.has_shadow = has_shadow;
    self
  }
}

pub trait EventLoopExtMacOS {
  /// Sets the activation policy for the application. It is set to
  /// `NSApplicationActivationPolicyRegular` by default.
  ///
  /// This function only takes effect if it's called before calling [`run`](crate::event_loop::EventLoop::run) or
  /// [`run_return`](crate::platform::run_return::EventLoopExtRunReturn::run_return)
  fn set_activation_policy(&mut self, activation_policy: ActivationPolicy);

  /// Used to prevent a default menubar menu from getting created
  ///
  /// The default menu creation is enabled by default.
  ///
  /// This function only takes effect if it's called before calling
  /// [`run`](crate::event_loop::EventLoop::run) or
  /// [`run_return`](crate::platform::run_return::EventLoopExtRunReturn::run_return)
  fn enable_default_menu_creation(&mut self, enable: bool);
}
impl<T> EventLoopExtMacOS for EventLoop<T> {
  #[inline]
  fn set_activation_policy(&mut self, activation_policy: ActivationPolicy) {
    unsafe {
      get_aux_state_mut(&**self.event_loop.delegate).activation_policy = activation_policy;
    }
  }

  #[inline]
  fn enable_default_menu_creation(&mut self, enable: bool) {
    unsafe {
      get_aux_state_mut(&**self.event_loop.delegate).create_default_menu = enable;
    }
  }
}

/// Additional methods on `MonitorHandle` that are specific to MacOS.
pub trait MonitorHandleExtMacOS {
  /// Returns the identifier of the monitor for Cocoa.
  fn native_id(&self) -> u32;
  /// Returns a pointer to the NSScreen representing this monitor.
  fn ns_screen(&self) -> Option<*mut c_void>;
}

impl MonitorHandleExtMacOS for MonitorHandle {
  #[inline]
  fn native_id(&self) -> u32 {
    self.inner.native_identifier()
  }

  fn ns_screen(&self) -> Option<*mut c_void> {
    self.inner.ns_screen().map(|s| s as *mut c_void)
  }
}

/// Additional methods on `EventLoopWindowTarget` that are specific to macOS.
pub trait EventLoopWindowTargetExtMacOS {
  /// Hide the entire application. In most applications this is typically triggered with Command-H.
  fn hide_application(&self);
  /// Show the entire application.
  fn show_application(&self);
  /// Hide the other applications. In most applications this is typically triggered with Command+Option-H.
  fn hide_other_applications(&self);
}

impl<T> EventLoopWindowTargetExtMacOS for EventLoopWindowTarget<T> {
  fn hide_application(&self) {
    let cls = objc::runtime::Class::get("NSApplication").unwrap();
    let app: cocoa::base::id = unsafe { msg_send![cls, sharedApplication] };
    unsafe { msg_send![app, hide: 0] }
  }

  fn show_application(&self) {
    let cls = objc::runtime::Class::get("NSApplication").unwrap();
    let app: cocoa::base::id = unsafe { msg_send![cls, sharedApplication] };
    unsafe { msg_send![app, unhide: 0] }
  }

  fn hide_other_applications(&self) {
    let cls = objc::runtime::Class::get("NSApplication").unwrap();
    let app: cocoa::base::id = unsafe { msg_send![cls, sharedApplication] };
    unsafe { msg_send![app, hideOtherApplications: 0] }
  }
}

#[cfg(feature = "tray")]
pub trait SystemTrayBuilderExtMacOS {
  /// Sets the icon as a [template](https://developer.apple.com/documentation/appkit/nsimage/1520017-template?language=objc).
  ///
  /// Images you mark as template images should consist of only black and clear colors.
  /// You can use the alpha channel in the image to adjust the opacity of black content.
  ///
  fn with_icon_as_template(self, is_template: bool) -> Self;
}

#[cfg(feature = "tray")]
impl SystemTrayBuilderExtMacOS for SystemTrayBuilder {
  fn with_icon_as_template(mut self, is_template: bool) -> Self {
    self.0.system_tray.icon_is_template = is_template;
    self
  }
}

#[cfg(feature = "tray")]
pub trait SystemTrayExtMacOS {
  /// Sets the icon as a [template](https://developer.apple.com/documentation/appkit/nsimage/1520017-template?language=objc).
  ///
  /// You need to update this value before changing the icon.
  ///
  fn set_icon_as_template(&mut self, is_template: bool);
}

#[cfg(feature = "tray")]
impl SystemTrayExtMacOS for SystemTray {
  fn set_icon_as_template(&mut self, is_template: bool) {
    self.0.icon_is_template = is_template
  }
}
