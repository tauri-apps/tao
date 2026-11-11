// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use super::{
  event_loop::EventLoopWindowTarget, monitor::MonitorHandle, window_delegate::WindowDelegate,
};
use crate::{
  dpi::{PhysicalPosition, PhysicalSize, Position, Size},
  error::{ExternalError, NotSupportedError, OsError as RootOsError},
  icon::Icon,
  monitor::MonitorHandle as RootMonitorHandle,
  platform::macos::WindowExtMacOS,
  window::{
    CursorIcon, Fullscreen, ProgressBarState, ResizeDirection, Theme, UserAttentionType,
    WindowAttributes, WindowSizeConstraints,
  },
};
use dispatch2::MainThreadBound;
use objc2::{
  define_class, msg_send,
  rc::{autoreleasepool, Retained},
  DefinedClass, MainThreadOnly,
};
use objc2_app_kit::{
  NSEvent, NSEventType, NSResponder, NSWindow, NSWindowButton, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSObject};
use std::{cell::Cell, collections::VecDeque, os::raw::c_void};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub usize);

impl Id {
  pub unsafe fn dummy() -> Self {
    Id(0)
  }
}

// Convert the `NSWindow` associated with a window to a usize to use as a unique identifier
// for the window.
pub fn get_window_id(window_cocoa_id: &NSWindow) -> Id {
  Id(window_cocoa_id as *const NSWindow as _)
}

#[derive(Debug)]
pub(super) struct WindowState {
  pub(super) focusable: Cell<bool>,
  pub(super) maximizable: Cell<bool>,
}

define_class!(
  #[unsafe(super(NSWindow, NSResponder, NSObject))]
  #[thread_kind = MainThreadOnly]
  #[name = "TaoWindow"]
  #[ivars = WindowState]
  #[derive(Debug)]
  pub(super) struct TaoWindow;

  impl TaoWindow {
    #[unsafe(method(setStyleMask:))]
    fn set_style_mask(&self, mask: NSWindowStyleMask) {
      unsafe {
        let _: () = msg_send![super(self), setStyleMask: mask];
        if let Some(button) = self.standardWindowButton(NSWindowButton::ZoomButton) {
          button.setEnabled(self.ivars().maximizable.get());
        }
      }
    }

    #[unsafe(method(canBecomeMainWindow))]
    fn can_become_main_window(&self) -> bool {
      self.ivars().focusable.get()
    }

    #[unsafe(method(canBecomeKeyWindow))]
    fn can_become_key_window(&self) -> bool {
      self.ivars().focusable.get()
    }

    #[unsafe(method(sendEvent:))]
    fn send_event(&self, event: &NSEvent) {
      unsafe {
        if event.r#type() == NSEventType::LeftMouseDown {
          // When WKWebView is set on NSWindow, movable_by_window_background
          // requires explicitly starting the window drag on a left mouse down.
          if self.isMovableByWindowBackground() {
            self.performWindowDragWithEvent(event);
          }
        }
        let _: () = msg_send![super(self), sendEvent: event];
      }
    }
  }
);

pub struct Window {
  // NSWindow keeps a weak delegate reference, so retain it for the window's lifetime.
  delegate: MainThreadBound<Retained<WindowDelegate>>,
}

impl Window {
  pub fn new<T: 'static>(
    _window_target: &EventLoopWindowTarget<T>,
    attributes: WindowAttributes,
    pl_attribs: super::PlatformSpecificWindowBuilderAttributes,
  ) -> Result<Self, RootOsError> {
    let mtm =
      MainThreadMarker::new().expect("Windows can only be created on the main thread on macOS");
    let delegate = autoreleasepool(|_| WindowDelegate::new(attributes, pl_attribs, mtm))?;
    Ok(Self {
      delegate: MainThreadBound::new(delegate, mtm),
    })
  }

  pub fn id(&self) -> Id {
    self.delegate.get_on_main(|delegate| delegate.id())
  }

  pub fn set_title(&self, title: &str) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_title(title))
  }

  pub fn title(&self) -> String {
    self.delegate.get_on_main(|delegate| delegate.title())
  }

  pub fn set_visible(&self, visible: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_visible(visible))
  }

  pub fn set_focus(&self) {
    self.delegate.get_on_main(|delegate| delegate.set_focus())
  }

  #[inline]
  pub fn set_focusable(&self, focusable: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_focusable(focusable))
  }

  #[inline]
  pub fn is_focused(&self) -> bool {
    self.delegate.get_on_main(|delegate| delegate.is_focused())
  }

  pub fn request_redraw(&self) {
    self
      .delegate
      .get_on_main(|delegate| delegate.request_redraw())
  }

  pub fn outer_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError> {
    self
      .delegate
      .get_on_main(|delegate| delegate.outer_position())
  }

  pub fn inner_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError> {
    self
      .delegate
      .get_on_main(|delegate| delegate.inner_position())
  }

  pub fn set_outer_position(&self, position: Position) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_outer_position(position))
  }

  #[inline]
  pub fn inner_size(&self) -> PhysicalSize<u32> {
    self.delegate.get_on_main(|delegate| delegate.inner_size())
  }

  #[inline]
  pub fn outer_size(&self) -> PhysicalSize<u32> {
    self.delegate.get_on_main(|delegate| delegate.outer_size())
  }

  #[inline]
  pub fn set_inner_size(&self, size: Size) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_inner_size(size))
  }

  pub fn set_min_inner_size(&self, dimensions: Option<Size>) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_min_inner_size(dimensions))
  }

  pub fn set_max_inner_size(&self, dimensions: Option<Size>) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_max_inner_size(dimensions))
  }

  pub fn set_inner_size_constraints(&self, constraints: WindowSizeConstraints) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_inner_size_constraints(constraints))
  }

  #[inline]
  pub fn set_resizable(&self, resizable: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_resizable(resizable))
  }

  #[inline]
  pub fn set_minimizable(&self, minimizable: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_minimizable(minimizable))
  }

  #[inline]
  pub fn set_maximizable(&self, maximizable: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_maximizable(maximizable))
  }

  #[inline]
  pub fn set_closable(&self, closable: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_closable(closable))
  }

  pub fn set_cursor_icon(&self, cursor: CursorIcon) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_cursor_icon(cursor))
  }

  #[inline]
  pub fn set_cursor_grab(&self, grab: bool) -> Result<(), ExternalError> {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_cursor_grab(grab))
  }

  #[inline]
  pub fn set_cursor_visible(&self, visible: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_cursor_visible(visible))
  }

  #[inline]
  pub fn cursor_position(&self) -> Result<PhysicalPosition<f64>, ExternalError> {
    self
      .delegate
      .get_on_main(|delegate| delegate.cursor_position())
  }

  #[inline]
  pub fn scale_factor(&self) -> f64 {
    self
      .delegate
      .get_on_main(|delegate| delegate.scale_factor())
  }

  #[inline]
  pub fn set_cursor_position(&self, cursor_position: Position) -> Result<(), ExternalError> {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_cursor_position(cursor_position))
  }

  #[inline]
  pub fn set_background_color(&self, color: Option<crate::window::RGBA>) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_background_color(color))
  }

  #[inline]
  pub fn drag_window(&self) -> Result<(), ExternalError> {
    self.delegate.get_on_main(|delegate| delegate.drag_window())
  }

  pub fn drag_resize_window(&self, _direction: ResizeDirection) -> Result<(), ExternalError> {
    self
      .delegate
      .get_on_main(|delegate| delegate.drag_resize_window(_direction))
  }

  #[inline]
  pub fn set_ignore_cursor_events(&self, ignore: bool) -> Result<(), ExternalError> {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_ignore_cursor_events(ignore))
  }

  #[inline]
  pub fn set_minimized(&self, minimized: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_minimized(minimized))
  }

  #[inline]
  pub fn set_maximized(&self, maximized: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_maximized(maximized))
  }

  #[inline]
  pub fn fullscreen(&self) -> Option<Fullscreen> {
    self.delegate.get_on_main(|delegate| delegate.fullscreen())
  }

  #[inline]
  pub fn is_visible(&self) -> bool {
    self.delegate.get_on_main(|delegate| delegate.is_visible())
  }

  #[inline]
  pub fn is_always_on_top(&self) -> bool {
    self
      .delegate
      .get_on_main(|delegate| delegate.is_always_on_top())
  }

  #[inline]
  pub fn is_maximized(&self) -> bool {
    self
      .delegate
      .get_on_main(|delegate| delegate.is_maximized())
  }

  #[inline]
  pub fn is_minimized(&self) -> bool {
    self
      .delegate
      .get_on_main(|delegate| delegate.is_minimized())
  }

  #[inline]
  pub fn is_resizable(&self) -> bool {
    self
      .delegate
      .get_on_main(|delegate| delegate.is_resizable())
  }

  #[inline]
  pub fn is_minimizable(&self) -> bool {
    self
      .delegate
      .get_on_main(|delegate| delegate.is_minimizable())
  }

  #[inline]
  pub fn is_maximizable(&self) -> bool {
    self
      .delegate
      .get_on_main(|delegate| delegate.is_maximizable())
  }

  #[inline]
  pub fn is_closable(&self) -> bool {
    self.delegate.get_on_main(|delegate| delegate.is_closable())
  }

  #[inline]
  pub fn is_decorated(&self) -> bool {
    self
      .delegate
      .get_on_main(|delegate| delegate.is_decorated())
  }

  #[inline]
  pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_fullscreen(fullscreen))
  }

  #[inline]
  pub fn set_decorations(&self, decorations: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_decorations(decorations))
  }

  #[inline]
  pub fn set_always_on_bottom(&self, always_on_bottom: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_always_on_bottom(always_on_bottom))
  }

  #[inline]
  pub fn set_always_on_top(&self, always_on_top: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_always_on_top(always_on_top))
  }

  #[inline]
  pub fn set_window_icon(&self, _icon: Option<Icon>) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_window_icon(_icon))
  }

  #[inline]
  pub fn set_ime_position(&self, spot: Position) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_ime_position(spot))
  }

  #[inline]
  pub fn request_user_attention(&self, request_type: Option<UserAttentionType>) {
    self
      .delegate
      .get_on_main(|delegate| delegate.request_user_attention(request_type))
  }

  #[inline]
  pub fn current_monitor(&self) -> Option<RootMonitorHandle> {
    self
      .delegate
      .get_on_main(|delegate| delegate.current_monitor())
  }

  #[inline]
  pub fn monitor_from_point(&self, x: f64, y: f64) -> Option<RootMonitorHandle> {
    self
      .delegate
      .get_on_main(|delegate| delegate.monitor_from_point(x, y))
  }

  #[inline]
  pub fn available_monitors(&self) -> VecDeque<MonitorHandle> {
    self
      .delegate
      .get_on_main(|delegate| delegate.available_monitors())
  }

  #[inline]
  pub fn primary_monitor(&self) -> Option<RootMonitorHandle> {
    self
      .delegate
      .get_on_main(|delegate| delegate.primary_monitor())
  }

  #[cfg(feature = "rwh_04")]
  #[inline]
  pub fn raw_window_handle_rwh_04(&self) -> rwh_04::RawWindowHandle {
    let mut window_handle = rwh_04::AppKitHandle::empty();
    window_handle.ns_window = self.ns_window();
    window_handle.ns_view = self.ns_view();
    rwh_04::RawWindowHandle::AppKit(window_handle)
  }

  #[cfg(feature = "rwh_05")]
  #[inline]
  pub fn raw_window_handle_rwh_05(&self) -> rwh_05::RawWindowHandle {
    let mut window_handle = rwh_05::AppKitWindowHandle::empty();
    window_handle.ns_window = self.ns_window();
    window_handle.ns_view = self.ns_view();
    rwh_05::RawWindowHandle::AppKit(window_handle)
  }

  #[cfg(feature = "rwh_05")]
  #[inline]
  pub fn raw_display_handle_rwh_05(&self) -> rwh_05::RawDisplayHandle {
    rwh_05::RawDisplayHandle::AppKit(rwh_05::AppKitDisplayHandle::empty())
  }

  #[cfg(feature = "rwh_06")]
  #[inline]
  pub fn raw_window_handle_rwh_06(&self) -> Result<rwh_06::RawWindowHandle, rwh_06::HandleError> {
    if MainThreadMarker::new().is_none() {
      return Err(rwh_06::HandleError::Unavailable);
    }
    let window_handle = rwh_06::AppKitWindowHandle::new({
      let ptr = self.ns_view();
      std::ptr::NonNull::new(ptr).expect("Id<T> should never be null")
    });
    Ok(rwh_06::RawWindowHandle::AppKit(window_handle))
  }

  #[cfg(feature = "rwh_06")]
  #[inline]
  pub fn raw_display_handle_rwh_06(&self) -> Result<rwh_06::RawDisplayHandle, rwh_06::HandleError> {
    Ok(rwh_06::RawDisplayHandle::AppKit(
      rwh_06::AppKitDisplayHandle::new(),
    ))
  }

  #[inline]
  pub fn theme(&self) -> Theme {
    self.delegate.get_on_main(|delegate| delegate.theme())
  }

  pub fn set_theme(&self, theme: Option<Theme>) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_theme(theme))
  }

  pub fn set_content_protection(&self, enabled: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_content_protection(enabled))
  }

  pub fn set_visible_on_all_workspaces(&self, visible: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_visible_on_all_workspaces(visible))
  }

  pub fn set_progress_bar(&self, progress: ProgressBarState) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_progress_bar(progress))
  }

  pub fn set_badge_label(&self, label: Option<String>) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_badge_label(label))
  }
}

impl WindowExtMacOS for Window {
  #[inline]
  fn ns_window(&self) -> *mut c_void {
    self
      .delegate
      .get_on_main(|delegate| delegate.ns_window() as usize) as *mut c_void
  }

  #[inline]
  fn ns_view(&self) -> *mut c_void {
    self
      .delegate
      .get_on_main(|delegate| delegate.ns_view() as usize) as *mut c_void
  }

  #[inline]
  fn simple_fullscreen(&self) -> bool {
    self
      .delegate
      .get_on_main(|delegate| delegate.simple_fullscreen())
  }

  #[inline]
  fn set_simple_fullscreen(&self, fullscreen: bool) -> bool {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_simple_fullscreen(fullscreen))
  }

  #[inline]
  fn has_shadow(&self) -> bool {
    self.delegate.get_on_main(|delegate| delegate.has_shadow())
  }

  #[inline]
  fn set_has_shadow(&self, has_shadow: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_has_shadow(has_shadow))
  }

  #[inline]
  fn set_traffic_light_inset<P: Into<Position>>(&self, position: P) {
    let position: Position = position.into();
    self
      .delegate
      .get_on_main(|delegate| delegate.set_traffic_light_inset(position))
  }

  #[inline]
  fn set_is_document_edited(&self, edited: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_is_document_edited(edited))
  }

  #[inline]
  fn is_document_edited(&self) -> bool {
    self
      .delegate
      .get_on_main(|delegate| delegate.is_document_edited())
  }

  #[inline]
  fn set_allows_automatic_window_tabbing(&self, enabled: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_allows_automatic_window_tabbing(enabled))
  }

  #[inline]
  fn allows_automatic_window_tabbing(&self) -> bool {
    self
      .delegate
      .get_on_main(|delegate| delegate.allows_automatic_window_tabbing())
  }

  #[inline]
  fn set_tabbing_identifier(&self, identifier: &str) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_tabbing_identifier(identifier))
  }

  #[inline]
  fn tabbing_identifier(&self) -> String {
    self
      .delegate
      .get_on_main(|delegate| delegate.tabbing_identifier())
  }

  #[inline]
  fn set_fullsize_content_view(&self, fullsize: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_fullsize_content_view(fullsize))
  }

  #[inline]
  fn set_titlebar_transparent(&self, transparent: bool) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_titlebar_transparent(transparent))
  }

  fn set_badge_label(&self, label: Option<String>) {
    self
      .delegate
      .get_on_main(|delegate| delegate.set_badge_label(label))
  }
}

impl Drop for Window {
  fn drop(&mut self) {
    self
      .delegate
      .get_on_main(|delegate| autoreleasepool(|_| delegate.close()));
  }
}
