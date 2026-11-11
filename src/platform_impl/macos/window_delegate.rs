// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  cell::{Cell, RefCell},
  collections::VecDeque,
  convert::TryInto,
  ffi::CStr,
  os::raw::c_void,
  path::PathBuf,
  sync::{Mutex, Weak},
};

use crate::{
  dpi::{
    LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position, Size, Size::Logical,
  },
  error::{ExternalError, NotSupportedError, OsError as RootOsError},
  event::{Event, WindowEvent},
  icon::Icon,
  keyboard::ModifiersState,
  monitor::{MonitorHandle as RootMonitorHandle, VideoMode as RootVideoMode},
  platform::macos::WindowExtMacOS,
  platform_impl::{
    platform::{
      app_state::AppState,
      ffi,
      monitor::{self, MonitorHandle, VideoMode},
      util::{self, IdRef},
      view::{self, new_view, CursorState},
      OsError,
    },
    set_badge_label, set_progress_indicator,
  },
  window::{
    CursorIcon, Fullscreen, ProgressBarState, ResizeDirection, Theme, UserAttentionType,
    WindowAttributes, WindowId, WindowId as RootWindowId, WindowSizeConstraints,
  },
};
use core_graphics::{base::CGFloat, display::CGDisplay, geometry::CGPoint};
use objc2::{
  define_class, msg_send,
  rc::{autoreleasepool, Retained},
  runtime::{AnyObject, ProtocolObject},
  ClassType, DefinedClass, MainThreadOnly, Message,
};
use objc2_app_kit::{
  self as appkit, NSApp, NSApplicationPresentationOptions, NSBackingStoreType, NSColor,
  NSDragOperation, NSDraggingDestination, NSDraggingInfo, NSEvent, NSEventSubtype, NSEventType,
  NSPasteboard, NSRequestUserAttentionType, NSScreen, NSView, NSWindow, NSWindowButton,
  NSWindowCollectionBehavior, NSWindowDelegate, NSWindowFullScreenButton, NSWindowOrderingMode,
  NSWindowSharingType, NSWindowStyleMask,
};
use objc2_foundation::{
  ns_string, MainThreadMarker, NSArray, NSAutoreleasePool, NSDictionary,
  NSDistributedNotificationCenter, NSInteger, NSNumber, NSObject, NSObjectNSDelayedPerforming,
  NSObjectNSThreadPerformAdditions, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use super::{
  event::{EventProxy, EventWrapper},
  ffi::{id, nil},
  view::{reapply_traffic_light_inset, TaoView},
  window::{get_window_id, Id, TaoWindow, WindowState},
};

#[non_exhaustive]
#[derive(Clone)]
pub enum Parent {
  None,
  ChildOf(*mut c_void),
}

#[derive(Clone)]
pub struct PlatformSpecificWindowBuilderAttributes {
  pub parent: Parent,
  pub movable_by_window_background: bool,
  pub titlebar_transparent: bool,
  pub title_hidden: bool,
  pub titlebar_hidden: bool,
  pub titlebar_buttons_hidden: bool,
  pub fullsize_content_view: bool,
  pub resize_increments: Option<LogicalSize<f64>>,
  pub disallow_hidpi: bool,
  pub has_shadow: bool,
  pub traffic_light_inset: Option<Position>,
  pub automatic_tabbing: bool,
  pub tabbing_identifier: Option<String>,
}

impl Default for PlatformSpecificWindowBuilderAttributes {
  #[inline]
  fn default() -> Self {
    Self {
      parent: Parent::None,
      movable_by_window_background: false,
      titlebar_transparent: false,
      title_hidden: false,
      titlebar_hidden: false,
      titlebar_buttons_hidden: false,
      fullsize_content_view: false,
      resize_increments: None,
      disallow_hidpi: false,
      has_shadow: true,
      traffic_light_inset: None,
      automatic_tabbing: true,
      tabbing_identifier: None,
    }
  }
}

unsafe fn create_view(
  ns_window: &NSWindow,
  pl_attribs: &PlatformSpecificWindowBuilderAttributes,
) -> (Retained<TaoView>, Weak<Mutex<CursorState>>) {
  let (ns_view, cursor_state) = new_view(ns_window);
  if !pl_attribs.disallow_hidpi {
    #[allow(deprecated)]
    ns_view.setWantsBestResolutionOpenGLSurface(true);
  }

  if let Some(position) = pl_attribs.traffic_light_inset {
    let state = ns_view.ivars();
    let scale_factor = NSWindow::backingScaleFactor(ns_window);
    let position = position.to_logical(scale_factor);
    state.traffic_light_inset.set(Some(position));
  }

  // On Mojave, views automatically become layer-backed shortly after being added to
  // a window. Changing the layer-backedness of a view breaks the association between
  // the view and its associated OpenGL context. To work around this, on Mojave we
  // explicitly make the view layer-backed up front so that AppKit doesn't do it
  // itself and break the association with its context.
  if f64::floor(appkit::NSAppKitVersionNumber) > appkit::NSAppKitVersionNumber10_12 {
    ns_view.setWantsLayer(true);
  }

  (ns_view, cursor_state)
}

fn create_window(
  attrs: &WindowAttributes,
  pl_attrs: &PlatformSpecificWindowBuilderAttributes,
) -> Retained<TaoWindow> {
  unsafe {
    let mtm = MainThreadMarker::new_unchecked();
    let _pool = NSAutoreleasePool::new();
    let screen = match attrs.fullscreen {
      Some(Fullscreen::Borderless(Some(RootMonitorHandle { inner: ref monitor })))
      | Some(Fullscreen::Exclusive(RootVideoMode {
        video_mode: VideoMode { ref monitor, .. },
      })) => {
        let monitor_screen = monitor.ns_screen();
        Some(monitor_screen.unwrap_or_else(|| appkit::NSScreen::mainScreen(mtm).unwrap()))
      }
      Some(Fullscreen::Borderless(None)) => Some(
        attrs
          .position
          .and_then(screen_from_position)
          .unwrap_or_else(|| appkit::NSScreen::mainScreen(mtm).unwrap()),
      ),
      None => None,
    };
    let frame = match &screen {
      Some(screen) => NSScreen::frame(screen),
      None => {
        let screen = attrs
          .position
          .and_then(screen_from_position)
          .unwrap_or_else(|| appkit::NSScreen::mainScreen(mtm).unwrap());
        let scale_factor = NSScreen::backingScaleFactor(&screen);
        let desired_size = attrs
          .inner_size
          .unwrap_or_else(|| PhysicalSize::new(800, 600).into());
        let (width, height): (f64, f64) = attrs
          .inner_size_constraints
          .clamp(desired_size, scale_factor)
          .to_logical::<f64>(scale_factor)
          .into();
        let (left, bottom) = match attrs.position {
          Some(position) => {
            let logical = util::window_position(position.to_logical(scale_factor));
            // macOS wants the position of the bottom left corner,
            // but caller is setting the position of top left corner
            (logical.x, logical.y - height)
          }
          // This value is ignored by calling win.center() below
          None => (0.0, 0.0),
        };
        NSRect::new(NSPoint::new(left, bottom), NSSize::new(width, height))
      }
    };

    let mut masks = if !attrs.decorations && screen.is_none() || pl_attrs.titlebar_hidden {
      // Resizable window without a titlebar or borders
      // if decorations is set to false, ignore pl_attrs
      NSWindowStyleMask::Borderless
        | NSWindowStyleMask::Resizable
        | NSWindowStyleMask::Miniaturizable
    } else {
      // default case, resizable window with titlebar and titlebar buttons
      NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable
        | NSWindowStyleMask::Titled
    };

    if !attrs.resizable {
      masks &= !NSWindowStyleMask::Resizable;
    }

    if !attrs.minimizable {
      masks &= !NSWindowStyleMask::Miniaturizable;
    }

    if !attrs.closable {
      masks &= !NSWindowStyleMask::Closable;
    }

    if pl_attrs.fullsize_content_view {
      masks |= NSWindowStyleMask::FullSizeContentView;
    }

    let state = WindowState {
      focusable: Cell::new(attrs.focusable),
      maximizable: Cell::new(attrs.maximizable),
    };
    let ns_window = TaoWindow::alloc(mtm).set_ivars(state);
    let ns_window: Retained<TaoWindow> = msg_send![
      super(ns_window),
      initWithContentRect: frame,
      styleMask: masks,
      backing: NSBackingStoreType::Buffered,
      defer: false,
    ];

    let title = NSString::from_str(&attrs.title);
    ns_window.setReleasedWhenClosed(false);
    ns_window.setTitle(&title);
    ns_window.setAcceptsMouseMovedEvents(true);

    if pl_attrs.titlebar_transparent {
      ns_window.setTitlebarAppearsTransparent(true);
    }
    if pl_attrs.title_hidden {
      ns_window.setTitleVisibility(appkit::NSWindowTitleVisibility::Hidden);
    }
    if pl_attrs.titlebar_buttons_hidden {
      for titlebar_button in &[
        NSWindowFullScreenButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::CloseButton,
        NSWindowButton::ZoomButton,
      ] {
        if let Some(button) = ns_window.standardWindowButton(*titlebar_button) {
          button.setHidden(true);
        }
      }
    }
    if pl_attrs.movable_by_window_background {
      ns_window.setMovableByWindowBackground(true);
    }

    if attrs.always_on_top {
      ns_window.setLevel(ffi::NSWindowLevel::NSFloatingWindowLevel as isize);
    }

    if attrs.always_on_bottom {
      ns_window.setLevel(ffi::NSWindowLevel::BelowNormalWindowLevel as isize);
    }

    if attrs.content_protection {
      ns_window.setSharingType(NSWindowSharingType::None);
    }

    if !attrs.maximizable {
      if let Some(button) = ns_window.standardWindowButton(NSWindowButton::ZoomButton) {
        button.setEnabled(false);
      }
    }

    if let Some(increments) = pl_attrs.resize_increments {
      let (x, y) = (increments.width, increments.height);
      if x >= 1.0 && y >= 1.0 {
        let size = NSSize::new(x as CGFloat, y as CGFloat);
        ns_window.setResizeIncrements(size);
      }
    }

    if let Parent::ChildOf(parent) = pl_attrs.parent {
      let _: () =
        msg_send![parent as id, addChildWindow: &*ns_window, ordered: NSWindowOrderingMode::Above];
    }

    if !pl_attrs.automatic_tabbing {
      NSWindow::setAllowsAutomaticWindowTabbing(false, mtm);
    }

    if let Some(tabbing_identifier) = &pl_attrs.tabbing_identifier {
      ns_window.setTabbingIdentifier(&NSString::from_str(tabbing_identifier));
    }

    if !pl_attrs.has_shadow {
      ns_window.setHasShadow(false);
    }
    if attrs.position.is_none() {
      ns_window.center();
    }

    ns_window
  }
}

fn screen_from_position(position: Position) -> Option<Retained<NSScreen>> {
  for m in monitor::available_monitors() {
    let monitor_pos = m.position();
    let monitor_size = m.size();

    // type annotations required for 32bit targets.
    let window_position = position.to_physical::<i32>(m.scale_factor());

    let is_in_monitor = monitor_pos.x <= window_position.x
      && window_position.x < monitor_pos.x + monitor_size.width as i32
      && monitor_pos.y <= window_position.y
      && window_position.y < monitor_pos.y + monitor_size.height as i32;
    if is_in_monitor {
      return m.ns_screen();
    }
  }
  None
}

pub(super) fn get_ns_theme() -> Theme {
  unsafe {
    let appearances: Vec<Retained<NSString>> = vec![
      NSString::from_str("NSAppearanceNameAqua"),
      NSString::from_str("NSAppearanceNameDarkAqua"),
    ];
    let app_class = class!(NSApplication);
    let app: id = msg_send![app_class, sharedApplication];
    let has_theme: bool = msg_send![app, respondsToSelector: sel!(effectiveAppearance)];
    if !has_theme {
      return Theme::Light;
    }
    let appearance: id = msg_send![app, effectiveAppearance];
    let name: Retained<NSString> = msg_send![
      appearance,
      bestMatchFromAppearancesWithNames: &*NSArray::from_retained_slice(&appearances)
    ];
    let name = name.to_string();
    match &*name {
      "NSAppearanceNameDarkAqua" => Theme::Dark,
      _ => Theme::Light,
    }
  }
}

pub(super) fn set_ns_theme(theme: Option<Theme>) {
  unsafe {
    let app_class = class!(NSApplication);
    let app: id = msg_send![app_class, sharedApplication];
    let has_theme: bool = msg_send![app, respondsToSelector: sel!(effectiveAppearance)];
    if has_theme {
      let appearance = if let Some(theme) = theme {
        let name = NSString::from_str(match theme {
          Theme::Dark => "NSAppearanceNameDarkAqua",
          Theme::Light => "NSAppearanceNameAqua",
        });
        msg_send![class!(NSAppearance), appearanceNamed: &*name]
      } else {
        nil
      };
      let _: () = msg_send![app, setAppearance: appearance];
    }
  }
}

pub(super) struct State {
  pub(super) ns_window: Retained<TaoWindow>, // never changes
  pub(super) ns_view: Retained<TaoView>,     // never changes
  input_context: IdRef,                      // never changes
  resizable: Cell<bool>,
  fullscreen: RefCell<Option<Fullscreen>>,
  // This is true between windowWillEnterFullScreen and windowDidEnterFullScreen
  // or windowWillExitFullScreen and windowDidExitFullScreen.
  // We must not toggle fullscreen when this is true.
  in_fullscreen_transition: Cell<bool>,
  // If it is attempted to toggle fullscreen when in_fullscreen_transition is true,
  // Set target_fullscreen and do after fullscreen transition is end.
  target_fullscreen: RefCell<Option<Option<Fullscreen>>>,
  maximized: Cell<bool>,
  standard_frame: Cell<Option<NSRect>>,
  is_simple_fullscreen: Cell<bool>,
  saved_style: Cell<Option<NSWindowStyleMask>>,
  /// Presentation options saved before entering `set_simple_fullscreen`, and
  /// restored upon exiting it. Also used when transitioning from Borderless to
  /// Exclusive fullscreen in `set_fullscreen` because we need to disable the menu
  /// bar in exclusive fullscreen but want to restore the original options when
  /// transitioning back to borderless fullscreen.
  save_presentation_opts: Cell<Option<NSApplicationPresentationOptions>>,
  current_theme: Cell<Theme>,
  decorations: Cell<bool>,
  cursor_state: Weak<Mutex<CursorState>>,
  transparent: bool,
  // This is set when WindowBuilder::with_fullscreen was set,
  // see comments of `window_did_fail_to_enter_fullscreen`
  initial_fullscreen: Cell<bool>,

  // During `windowDidResize`, we use this to only send Moved if the position changed.
  previous_position: Cell<Option<(f64, f64)>>,

  // Used to prevent redundant events.
  previous_scale_factor: Cell<f64>,

  // Used to prevent resized events from being fired
  // when we are using our workaround in the `is_zoomed` function.
  is_checking_zoomed_in: Cell<bool>,
}
impl WindowDelegate {
  pub(super) fn new(
    mut win_attribs: WindowAttributes,
    pl_attribs: PlatformSpecificWindowBuilderAttributes,
    mtm: MainThreadMarker,
  ) -> Result<Retained<Self>, RootOsError> {
    trace!("Creating new window");

    let _pool = unsafe { NSAutoreleasePool::new() };

    let ns_window = create_window(&win_attribs, &pl_attribs);
    let (ns_view, cursor_state) = unsafe { create_view(&ns_window, &pl_attribs) };

    // Configure the new view as the "key view" for the window
    unsafe {
      ns_window.setContentView(Some(&ns_view));
      ns_window.setInitialFirstResponder(Some(&ns_view));
    }

    let input_context = unsafe { util::create_input_context(&ns_view) };

    let scale_factor = unsafe { NSWindow::backingScaleFactor(&ns_window) };

    unsafe {
      if win_attribs.transparent {
        ns_window.setOpaque(false);
      }

      if win_attribs.transparent || win_attribs.background_color.is_some() {
        let color = win_attribs
          .background_color
          .map(|(r, g, b, a)| {
            NSColor::colorWithRed_green_blue_alpha(
              r as f64 / 255.0,
              g as f64 / 255.0,
              b as f64 / 255.0,
              a as f64 / 255.0,
            )
          })
          .unwrap_or_else(|| NSColor::clearColor());
        ns_window.setBackgroundColor(Some(&color));
      }

      if win_attribs.inner_size_constraints.has_min() {
        let min_size = win_attribs
          .inner_size_constraints
          .min_size_logical(scale_factor);
        set_min_inner_size(&ns_window, min_size);
      }
      if win_attribs.inner_size_constraints.has_max() {
        let max_size = win_attribs
          .inner_size_constraints
          .max_size_logical(scale_factor);
        set_max_inner_size(&ns_window, max_size);
      }

      // register for drag and drop operations.
      ns_window.registerForDraggedTypes(&NSArray::arrayWithObject(appkit::NSFilenamesPboardType));
    }

    // Fullscreen state is set
    // indirectly by us calling `set_fullscreen` below, causing handlers in
    // `WindowDelegate` to update the state.
    let fullscreen = win_attribs.fullscreen.take();
    let maximized = win_attribs.maximized;
    let transparent = win_attribs.transparent;
    let visible = win_attribs.visible;
    let focused = win_attribs.focused;
    let decorations = win_attribs.decorations;
    let visible_on_all_workspaces = win_attribs.visible_on_all_workspaces;

    let cloned_preferred_theme = win_attribs.preferred_theme;

    let state = State {
      ns_view,
      ns_window: ns_window.retain(),
      input_context,
      resizable: Cell::new(win_attribs.resizable),
      // The window has not entered fullscreen yet. Initial fullscreen is applied below.
      fullscreen: RefCell::new(None),
      in_fullscreen_transition: Cell::new(false),
      target_fullscreen: RefCell::new(None),
      maximized: Cell::new(maximized),
      standard_frame: Cell::new(None),
      is_simple_fullscreen: Cell::new(false),
      saved_style: Cell::new(None),
      save_presentation_opts: Cell::new(None),
      current_theme: Cell::new(Theme::default()),
      decorations: Cell::new(decorations),
      cursor_state,
      transparent,
      initial_fullscreen: Cell::new(fullscreen.is_some()),
      previous_position: Cell::new(None),
      previous_scale_factor: Cell::new(scale_factor),
      is_checking_zoomed_in: Cell::new(false),
    };
    let delegate = Self::alloc(mtm).set_ivars(state);
    let delegate: Retained<Self> = unsafe { msg_send![super(delegate), init] };
    let window = delegate.ivars();

    match cloned_preferred_theme {
      Some(theme) => {
        set_ns_theme(Some(theme));
        window.current_theme.set(theme);
      }
      None => {
        window.current_theme.set(get_ns_theme());
      }
    }

    ns_window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    unsafe {
      let notification_center = NSDistributedNotificationCenter::defaultCenter();
      let notification_name = ns_string!("AppleInterfaceThemeChangedNotification");
      notification_center.addObserver_selector_name_object(
        &delegate,
        sel!(effectiveAppearanceDidChange:),
        Some(notification_name),
        None,
      );
    }
    if (scale_factor - 1.0).abs() > f64::EPSILON {
      delegate.emit_static_scale_factor_changed_event();
    }

    // Set fullscreen mode after we setup everything
    delegate.set_fullscreen(fullscreen);
    delegate.set_visible_on_all_workspaces(visible_on_all_workspaces);

    // Setting the window as key has to happen *after* we set the fullscreen
    // state, since otherwise we'll briefly see the window at normal size
    // before it transitions.
    if visible {
      if focused {
        // Tightly linked with `app_state::window_activation_hack`
        unsafe { ns_window.makeKeyAndOrderFront(None) };
      } else {
        unsafe { ns_window.orderFront(None) };
      }
    }

    if maximized {
      delegate.set_maximized(maximized);
    }

    Ok(delegate)
  }
}

impl WindowDelegate {
  pub(super) fn close(&self) {
    self.ivars().ns_window.close();
  }

  fn toggle_fullscreen(&self) {
    unsafe {
      let curr_mask = self.ivars().ns_window.styleMask();
      let required = NSWindowStyleMask::Titled | NSWindowStyleMask::Resizable;
      if self.ivars().fullscreen.borrow().is_some() && !curr_mask.contains(required) {
        self.set_style_mask(required);
        self.ivars().saved_style.set(Some(curr_mask));
      }
      self
        .ivars()
        .ns_window
        .setLevel(objc2_app_kit::NSNormalWindowLevel);
      self.ivars().ns_window.toggleFullScreen(None);
    }
  }

  fn set_style_mask(&self, mask: NSWindowStyleMask) {
    let ns_window = &*self.ivars().ns_window;
    unsafe {
      ns_window.setStyleMask(mask);
      // Restore keyboard handling after changing the window style.
      ns_window.makeFirstResponder(Some(&self.ivars().ns_view));
    }
  }

  pub fn id(&self) -> Id {
    let ns_window = &*self.ivars().ns_window;
    get_window_id(ns_window)
  }

  pub fn set_title(&self, title: &str) {
    let ns_window = &*self.ivars().ns_window;
    unsafe {
      ns_window.setTitle(&NSString::from_str(title));
      // Setting the title resets the traffic light buttons to their default position.
      view::reapply_traffic_light_inset(ns_window, &self.ivars().ns_view);
    }
  }

  pub fn title(&self) -> String {
    let ns_window = &*self.ivars().ns_window;
    ns_window.title().to_string()
  }

  pub fn set_visible(&self, visible: bool) {
    let ns_window = &*self.ivars().ns_window;
    match visible {
      true => unsafe { ns_window.makeKeyAndOrderFront(None) },
      false => unsafe { ns_window.orderOut(None) },
    }
  }

  #[inline]
  // Shortener for set_visible(true)
  pub fn set_focus(&self) {
    let ns_window = &*self.ivars().ns_window;
    unsafe {
      let is_minimized = ns_window.isMiniaturized();
      let is_visible = ns_window.isVisible();
      if !is_minimized && is_visible {
        ns_window.makeKeyAndOrderFront(None);
        let mtm = MainThreadMarker::from(ns_window.as_super());
        #[allow(deprecated)]
        NSApp(mtm).activateIgnoringOtherApps(true);
      }
    }
  }

  #[inline]
  pub fn set_focusable(&self, focusable: bool) {
    let ns_window = &*self.ivars().ns_window;

    ns_window.ivars().focusable.set(focusable);
  }

  #[inline]
  pub fn is_focused(&self) -> bool {
    let ns_window = &*self.ivars().ns_window;
    unsafe { ns_window.isKeyWindow() }
  }

  pub fn request_redraw(&self) {
    AppState::queue_redraw(RootWindowId(self.id()));
  }

  pub fn outer_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError> {
    let ns_window = &*self.ivars().ns_window;

    let frame_rect = unsafe { NSWindow::frame(ns_window) };
    let position = LogicalPosition::new(
      frame_rect.origin.x,
      util::bottom_left_to_top_left(frame_rect),
    );
    let scale_factor = self.scale_factor();
    Ok(position.to_physical(scale_factor))
  }

  pub fn inner_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError> {
    let ns_window = &*self.ivars().ns_window;

    let content_rect =
      unsafe { NSWindow::contentRectForFrameRect(ns_window, NSWindow::frame(ns_window)) };
    let position = LogicalPosition::new(
      content_rect.origin.x,
      util::bottom_left_to_top_left(content_rect),
    );
    let scale_factor = self.scale_factor();
    Ok(position.to_physical(scale_factor))
  }

  pub fn set_outer_position(&self, position: Position) {
    let ns_window = &*self.ivars().ns_window;

    let scale_factor = self.scale_factor();
    let position = position.to_logical(scale_factor);
    unsafe {
      ns_window.setFrameTopLeftPoint(util::window_position(position));
    }
  }

  #[inline]
  pub fn inner_size(&self) -> PhysicalSize<u32> {
    let ns_view = &*self.ivars().ns_view;

    let view_frame = unsafe { NSView::frame(ns_view) };
    let logical: LogicalSize<f64> = (view_frame.size.width, view_frame.size.height).into();
    logical.to_physical(self.scale_factor())
  }

  #[inline]
  pub fn outer_size(&self) -> PhysicalSize<u32> {
    let ns_window = &*self.ivars().ns_window;

    let view_frame = unsafe { NSWindow::frame(ns_window) };
    let logical: LogicalSize<f64> = (view_frame.size.width, view_frame.size.height).into();
    let scale_factor = self.scale_factor();
    logical.to_physical(scale_factor)
  }

  #[inline]
  pub fn set_inner_size(&self, size: Size) {
    let ns_window = &*self.ivars().ns_window;
    unsafe {
      let scale_factor = self.scale_factor();
      let size = size.to_logical::<CGFloat>(scale_factor);
      ns_window.setContentSize(NSSize::new(size.width, size.height));
    }
  }

  pub fn set_min_inner_size(&self, dimensions: Option<Size>) {
    let ns_window = &*self.ivars().ns_window;

    let dimensions = dimensions.unwrap_or(Logical(LogicalSize {
      width: 0.0,
      height: 0.0,
    }));
    let scale_factor = self.scale_factor();
    unsafe {
      set_min_inner_size(ns_window, dimensions.to_logical(scale_factor));
    }
  }

  pub fn set_max_inner_size(&self, dimensions: Option<Size>) {
    let ns_window = &*self.ivars().ns_window;

    let dimensions = dimensions.unwrap_or(Logical(LogicalSize {
      width: f32::MAX as f64,
      height: f32::MAX as f64,
    }));
    let scale_factor = self.scale_factor();
    unsafe {
      set_max_inner_size(ns_window, dimensions.to_logical(scale_factor));
    }
  }

  pub fn set_inner_size_constraints(&self, constraints: WindowSizeConstraints) {
    let ns_window = &*self.ivars().ns_window;

    let scale_factor = self.scale_factor();
    unsafe {
      let min_size = constraints.min_size_logical(scale_factor);
      set_min_inner_size(ns_window, min_size);
      let max_size = constraints.max_size_logical(scale_factor);
      set_max_inner_size(ns_window, max_size);
    }
  }

  #[inline]
  pub fn set_resizable(&self, resizable: bool) {
    let ns_window = &*self.ivars().ns_window;

    self.ivars().resizable.set(resizable);
    let fullscreen = self.ivars().fullscreen.borrow().is_some();
    if !fullscreen {
      let mut mask = unsafe { ns_window.styleMask() };
      if resizable {
        mask |= NSWindowStyleMask::Resizable;
      } else {
        mask &= !NSWindowStyleMask::Resizable;
      }
      self.set_style_mask(mask);
    } // Otherwise, we don't change the mask until we exit fullscreen.
  }

  #[inline]
  pub fn set_minimizable(&self, minimizable: bool) {
    let ns_window = &*self.ivars().ns_window;

    let mut mask = unsafe { ns_window.styleMask() };
    if minimizable {
      mask |= NSWindowStyleMask::Miniaturizable;
    } else {
      mask &= !NSWindowStyleMask::Miniaturizable;
    }
    self.set_style_mask(mask);
  }

  #[inline]
  pub fn set_maximizable(&self, maximizable: bool) {
    let ns_window = &*self.ivars().ns_window;

    ns_window.ivars().maximizable.set(maximizable);
    unsafe {
      if let Some(button) = ns_window.standardWindowButton(NSWindowButton::ZoomButton) {
        button.setEnabled(maximizable);
      }
    }
  }

  #[inline]
  pub fn set_closable(&self, closable: bool) {
    let ns_window = &*self.ivars().ns_window;

    let mut mask = unsafe { ns_window.styleMask() };
    if closable {
      mask |= NSWindowStyleMask::Closable;
    } else {
      mask &= !NSWindowStyleMask::Closable;
    }
    self.set_style_mask(mask);
  }

  pub fn set_cursor_icon(&self, cursor: CursorIcon) {
    let ns_window = &*self.ivars().ns_window;

    let cursor = util::Cursor::from(cursor);
    if let Some(cursor_access) = self.ivars().cursor_state.upgrade() {
      cursor_access.lock().unwrap().cursor = cursor;
    }
    unsafe {
      ns_window.invalidateCursorRectsForView(&self.ivars().ns_view);
    }
  }

  #[inline]
  pub fn set_cursor_grab(&self, grab: bool) -> Result<(), ExternalError> {
    // TODO: Do this for real https://stackoverflow.com/a/40922095/5435443
    CGDisplay::associate_mouse_and_mouse_cursor_position(!grab)
      .map_err(|status| ExternalError::Os(os_error!(OsError::CGError(status))))
  }

  #[inline]
  pub fn set_cursor_visible(&self, visible: bool) {
    let ns_window = &*self.ivars().ns_window;

    if let Some(cursor_access) = self.ivars().cursor_state.upgrade() {
      let mut cursor_state = cursor_access.lock().unwrap();
      if visible != cursor_state.visible {
        cursor_state.visible = visible;
        drop(cursor_state);
        unsafe {
          ns_window.invalidateCursorRectsForView(&self.ivars().ns_view);
        }
      }
    }
  }

  #[inline]
  pub fn cursor_position(&self) -> Result<PhysicalPosition<f64>, ExternalError> {
    util::cursor_position()
  }

  #[inline]
  pub fn scale_factor(&self) -> f64 {
    let ns_window = &*self.ivars().ns_window;
    NSWindow::backingScaleFactor(ns_window) as _
  }

  #[inline]
  pub fn set_cursor_position(&self, cursor_position: Position) -> Result<(), ExternalError> {
    let physical_window_position = self.inner_position().unwrap();
    let scale_factor = self.scale_factor();
    let window_position = physical_window_position.to_logical::<CGFloat>(scale_factor);
    let logical_cursor_position = cursor_position.to_logical::<CGFloat>(scale_factor);
    let point = CGPoint {
      x: logical_cursor_position.x + window_position.x,
      y: logical_cursor_position.y + window_position.y,
    };
    CGDisplay::warp_mouse_cursor_position(point)
      .map_err(|e| ExternalError::Os(os_error!(OsError::CGError(e))))?;
    CGDisplay::associate_mouse_and_mouse_cursor_position(true)
      .map_err(|e| ExternalError::Os(os_error!(OsError::CGError(e))))?;

    Ok(())
  }

  #[inline]
  pub fn set_background_color(&self, color: Option<crate::window::RGBA>) {
    let ns_window = &*self.ivars().ns_window;
    unsafe {
      let color = color
        .map(|(r, g, b, a)| {
          Some(NSColor::colorWithRed_green_blue_alpha(
            r as f64 / 255.0,
            g as f64 / 255.0,
            b as f64 / 255.0,
            a as f64 / 255.0,
          ))
        })
        .unwrap_or_else(|| {
          if self.ivars().transparent {
            Some(NSColor::clearColor())
          } else {
            None
          }
        });
      ns_window.setBackgroundColor(color.as_deref());
    }
  }

  #[inline]
  pub fn drag_window(&self) -> Result<(), ExternalError> {
    let ns_window = &*self.ivars().ns_window;

    unsafe {
      let mtm = MainThreadMarker::from(ns_window.as_super());
      let Some(mut event) = NSApp(mtm).currentEvent() else {
        // `currentEvent` is nil when no event is being dispatched, e.g. when the
        // drag is requested asynchronously after the mouse-down was handled.
        log::warn!("Cannot drag the window: there is no current event");
        return Ok(());
      };

      if event.r#type().0 == 0x15 {
        event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
            NSEventType::LeftMouseDown,
            NSEvent::mouseLocation(),
            event.modifierFlags(),
            event.timestamp(),
            event.windowNumber(),
            None,
            NSEventSubtype::WindowExposed.0 as NSInteger,
            1,
            1.0,
          ).unwrap();
      }

      ns_window.performWindowDragWithEvent(&event);
    }

    Ok(())
  }

  pub fn drag_resize_window(&self, _direction: ResizeDirection) -> Result<(), ExternalError> {
    Err(ExternalError::NotSupported(NotSupportedError::new()))
  }

  #[inline]
  pub fn set_ignore_cursor_events(&self, ignore: bool) -> Result<(), ExternalError> {
    let ns_window = &*self.ivars().ns_window;

    unsafe {
      ns_window.setIgnoresMouseEvents(ignore);
    }

    Ok(())
  }

  pub(crate) fn is_zoomed(&self) -> bool {
    let ns_window = &*self.ivars().ns_window;

    // Previously, is_zoomed temporarily mutated the window's styleMask(or resizable)
    // to force macOS to return a valid result for borderless windows.
    // This synchronous mutation could trigger unnecessary layout passes or state inconsistencies.
    // Related issue: https://github.com/rust-windowing/winit/issues/4071 and https://github.com/tauri-apps/plugins-workspace/issues/3240

    unsafe {
      let curr_mask = ns_window.styleMask();

      let required = NSWindowStyleMask::Titled | NSWindowStyleMask::Resizable;
      if curr_mask.contains(required) {
        return ns_window.isZoomed();
      }

      if let Some(screen) = ns_window.screen() {
        let frame = ns_window.frame();
        let visible_frame = screen.visibleFrame();

        return (frame.size.width - visible_frame.size.width).abs() < 1.0
          && (frame.size.height - visible_frame.size.height).abs() < 1.0;
      }

      // Fallback to original `isZoomed` check
      ns_window.isZoomed()
    }
  }

  fn saved_style(&self) -> NSWindowStyleMask {
    let ns_window = &*self.ivars().ns_window;

    let base_mask = self
      .ivars()
      .saved_style
      .take()
      .unwrap_or_else(|| unsafe { ns_window.styleMask() });
    if self.ivars().resizable.get() {
      base_mask | NSWindowStyleMask::Resizable
    } else {
      base_mask & !NSWindowStyleMask::Resizable
    }
  }

  /// This is called when the window is exiting fullscreen, whether by the
  /// user clicking on the green fullscreen button or programmatically by
  /// `toggleFullScreen:`
  pub(crate) fn restore_state_from_fullscreen(&self) {
    self.ivars().fullscreen.replace(None);

    let maximized = self.ivars().maximized.get();
    let mask = self.saved_style();

    self.set_style_mask(mask);
    self.set_maximized(maximized);
  }

  #[inline]
  pub fn set_minimized(&self, minimized: bool) {
    let ns_window = &*self.ivars().ns_window;

    let is_minimized: bool = unsafe { ns_window.isMiniaturized() };
    if is_minimized == minimized {
      return;
    }

    if minimized {
      unsafe {
        NSWindow::miniaturize(ns_window, Some(ns_window));
      }
    } else {
      unsafe {
        NSWindow::deminiaturize(ns_window, Some(ns_window));
      }
    }
  }

  #[inline]
  pub fn set_maximized(&self, maximized: bool) {
    let ns_window = &*self.ivars().ns_window;

    let is_zoomed = self.is_zoomed();
    if is_zoomed == maximized {
      return;
    };
    unsafe {
      if !is_zoomed {
        self.ivars().standard_frame.set(Some(ns_window.frame()));
      }
      self.ivars().maximized.set(maximized);
      if self.ivars().fullscreen.borrow().is_some() {
        // Apply the requested state when native fullscreen ends.
        return;
      }
      let standard_frame = self
        .ivars()
        .standard_frame
        .get()
        .unwrap_or(DEFAULT_STANDARD_FRAME);

      let required = NSWindowStyleMask::Resizable | NSWindowStyleMask::Titled;
      if ns_window.styleMask().contains(required) {
        ns_window.zoom(None);
      } else {
        let frame = if maximized {
          let mtm = MainThreadMarker::from(ns_window.as_super());
          NSScreen::mainScreen(mtm).unwrap().visibleFrame()
        } else {
          standard_frame
        };
        ns_window.setFrame_display_animate(frame, false, true);
      }
    }
  }

  #[inline]
  pub fn fullscreen(&self) -> Option<Fullscreen> {
    self.ivars().fullscreen.borrow().clone()
  }

  #[inline]
  pub fn is_visible(&self) -> bool {
    let ns_window = &*self.ivars().ns_window;
    unsafe { ns_window.isVisible() }
  }

  #[inline]
  pub fn is_always_on_top(&self) -> bool {
    let ns_window = &*self.ivars().ns_window;
    unsafe { ns_window.level() == ffi::kCGFloatingWindowLevelKey }
  }

  #[inline]
  pub fn is_maximized(&self) -> bool {
    self.is_zoomed()
  }

  #[inline]
  pub fn is_minimized(&self) -> bool {
    let ns_window = &*self.ivars().ns_window;
    unsafe { ns_window.isMiniaturized() }
  }

  #[inline]
  pub fn is_resizable(&self) -> bool {
    let ns_window = &*self.ivars().ns_window;
    ns_window.isResizable()
  }

  #[inline]
  pub fn is_minimizable(&self) -> bool {
    let ns_window = &*self.ivars().ns_window;
    ns_window.isMiniaturizable()
  }

  #[inline]
  pub fn is_maximizable(&self) -> bool {
    let ns_window = &*self.ivars().ns_window;
    unsafe {
      if let Some(button) = ns_window.standardWindowButton(NSWindowButton::ZoomButton) {
        button.isEnabled()
      } else {
        ns_window.ivars().maximizable.get()
      }
    }
  }

  #[inline]
  pub fn is_closable(&self) -> bool {
    let ns_window = &*self.ivars().ns_window;
    ns_window.hasCloseBox()
  }

  #[inline]
  pub fn is_decorated(&self) -> bool {
    self.ivars().decorations.get()
  }

  #[inline]
  pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
    let ns_window = &*self.ivars().ns_window;

    let mtm = MainThreadMarker::from(ns_window.as_super());

    if self.ivars().is_simple_fullscreen.get() {
      return;
    }
    if self.ivars().in_fullscreen_transition.get() {
      // We can't set fullscreen here.
      // Set fullscreen after transition.
      self.ivars().target_fullscreen.replace(Some(fullscreen));

      return;
    }
    let old_fullscreen = self.ivars().fullscreen.borrow().clone();
    if fullscreen == old_fullscreen {
      return;
    }

    // If the fullscreen is on a different monitor, we must move the window
    // to that monitor before we toggle fullscreen (as `toggleFullScreen`
    // does not take a screen parameter, but uses the current screen)
    if let Some(ref fullscreen) = fullscreen {
      let new_screen = match fullscreen {
        Fullscreen::Borderless(Some(borderless)) => {
          let RootMonitorHandle { inner: monitor } = borderless.clone();
          monitor
        }
        Fullscreen::Borderless(None) => {
          if let Some(current) = self.current_monitor_inner() {
            let RootMonitorHandle { inner: monitor } = current;
            monitor
          } else {
            return;
          }
        }
        Fullscreen::Exclusive(RootVideoMode {
          video_mode: VideoMode { ref monitor, .. },
        }) => monitor.clone(),
      }
      .ns_screen()
      .unwrap();

      unsafe {
        let old_screen = NSWindow::screen(ns_window);
        if old_screen.as_ref() != Some(&new_screen) {
          let mut screen_frame = new_screen.frame();
          // The coordinate system here has its origin at bottom-left
          // and Y goes up
          screen_frame.origin.y += screen_frame.size.height;
          ns_window.setFrameTopLeftPoint(screen_frame.origin);
        }
      }
    }

    if let Some(Fullscreen::Exclusive(ref video_mode)) = fullscreen {
      // Note: `enterFullScreenMode:withOptions:` seems to do the exact
      // same thing as we're doing here (captures the display, sets the
      // video mode, and hides the menu bar and dock), with the exception
      // of that I couldn't figure out how to set the display mode with
      // it. I think `enterFullScreenMode:withOptions:` is still using the
      // older display mode API where display modes were of the type
      // `CFDictionary`, but this has changed, so we can't obtain the
      // correct parameter for this any longer. Apple's code samples for
      // this function seem to just pass in "YES" for the display mode
      // parameter, which is not consistent with the docs saying that it
      // takes a `NSDictionary`..

      let display_id = video_mode.monitor().inner.native_identifier();

      let mut fade_token = ffi::kCGDisplayFadeReservationInvalidToken;

      if matches!(old_fullscreen, Some(Fullscreen::Borderless(_))) {
        unsafe {
          let app = NSApp(mtm);

          self
            .ivars()
            .save_presentation_opts
            .set(Some(app.presentationOptions()));
        }
      }

      unsafe {
        // Fade to black (and wait for the fade to complete) to hide the
        // flicker from capturing the display and switching display mode
        if ffi::CGAcquireDisplayFadeReservation(5.0, &mut fade_token) == ffi::kCGErrorSuccess {
          ffi::CGDisplayFade(
            fade_token,
            0.3,
            ffi::kCGDisplayBlendNormal,
            ffi::kCGDisplayBlendSolidColor,
            0.0,
            0.0,
            0.0,
            ffi::TRUE,
          );
        }

        assert_eq!(ffi::CGDisplayCapture(display_id), ffi::kCGErrorSuccess);
      }

      unsafe {
        let result = ffi::CGDisplaySetDisplayMode(
          display_id,
          video_mode.video_mode.native_mode.0,
          std::ptr::null(),
        );
        assert!(result == ffi::kCGErrorSuccess, "failed to set video mode");

        // After the display has been configured, fade back in
        // asynchronously
        if fade_token != ffi::kCGDisplayFadeReservationInvalidToken {
          ffi::CGDisplayFade(
            fade_token,
            0.6,
            ffi::kCGDisplayBlendSolidColor,
            ffi::kCGDisplayBlendNormal,
            0.0,
            0.0,
            0.0,
            ffi::FALSE,
          );
          ffi::CGReleaseDisplayFadeReservation(fade_token);
        }
      }
    }

    self.ivars().fullscreen.replace(fullscreen.clone());

    match (&old_fullscreen, &fullscreen) {
      (&None, &Some(_)) => unsafe {
        self.toggle_fullscreen();
      },
      (&Some(Fullscreen::Borderless(_)), &None) => unsafe {
        // State is restored by `window_did_exit_fullscreen`
        self.toggle_fullscreen();
      },
      (&Some(Fullscreen::Exclusive(RootVideoMode { ref video_mode })), &None) => unsafe {
        util::restore_display_mode(video_mode.monitor().inner.native_identifier());
        // Rest of the state is restored by `window_did_exit_fullscreen`
        self.toggle_fullscreen();
      },
      (&Some(Fullscreen::Borderless(_)), &Some(Fullscreen::Exclusive(_))) => unsafe {
        // If we're already in fullscreen mode, calling
        // `CGDisplayCapture` will place the shielding window on top of
        // our window, which results in a black display and is not what
        // we want. So, we must place our window on top of the shielding
        // window. Unfortunately, this also makes our window be on top
        // of the menu bar, and this looks broken, so we must make sure
        // that the menu bar is disabled. This is done in the window
        // delegate in `window:willUseFullScreenPresentationOptions:`.
        let app = NSApp(mtm);

        self
          .ivars()
          .save_presentation_opts
          .set(Some(app.presentationOptions()));

        let presentation_options = NSApplicationPresentationOptions::FullScreen
          | NSApplicationPresentationOptions::HideDock
          | NSApplicationPresentationOptions::HideMenuBar;
        app.setPresentationOptions(presentation_options);

        ns_window.setLevel(ffi::CGShieldingWindowLevel() as isize + 1);
      },
      (
        &Some(Fullscreen::Exclusive(RootVideoMode { ref video_mode })),
        &Some(Fullscreen::Borderless(_)),
      ) => unsafe {
        let presentation_options = self
          .ivars()
          .save_presentation_opts
          .get()
          .unwrap_or_else(|| {
            NSApplicationPresentationOptions::FullScreen
              | NSApplicationPresentationOptions::AutoHideDock
              | NSApplicationPresentationOptions::AutoHideMenuBar
          });
        NSApp(mtm).setPresentationOptions(presentation_options);

        util::restore_display_mode(video_mode.monitor().inner.native_identifier());

        // Restore the normal window level following the Borderless fullscreen
        // `CGShieldingWindowLevel() + 1` hack.
        ns_window.setLevel(objc2_app_kit::NSNormalWindowLevel);
      },
      _ => {}
    }
  }

  #[inline]
  pub fn set_decorations(&self, decorations: bool) {
    if decorations != self.ivars().decorations.get() {
      self.ivars().decorations.set(decorations);

      let fullscreen = self.ivars().fullscreen.borrow().is_some();
      let resizable = self.ivars().resizable.get();

      // If we're in fullscreen mode, we wait to apply decoration changes
      // until we're in `window_did_exit_fullscreen`.
      if fullscreen {
        return;
      }

      let new_mask = {
        let mut new_mask = if decorations {
          NSWindowStyleMask::Closable
            | NSWindowStyleMask::Miniaturizable
            | NSWindowStyleMask::Resizable
            | NSWindowStyleMask::Titled
        } else {
          NSWindowStyleMask::Borderless | NSWindowStyleMask::Resizable
        };
        if !resizable {
          new_mask &= !NSWindowStyleMask::Resizable;
        }
        new_mask
      };
      self.set_style_mask(new_mask);
    }
  }

  #[inline]
  pub fn set_always_on_bottom(&self, always_on_bottom: bool) {
    let ns_window = &*self.ivars().ns_window;

    let level = if always_on_bottom {
      ffi::NSWindowLevel::BelowNormalWindowLevel
    } else {
      ffi::NSWindowLevel::NSNormalWindowLevel
    };
    unsafe { ns_window.setLevel(level as _) };
  }

  #[inline]
  pub fn set_always_on_top(&self, always_on_top: bool) {
    let ns_window = &*self.ivars().ns_window;

    let level = if always_on_top {
      ffi::NSWindowLevel::NSFloatingWindowLevel
    } else {
      ffi::NSWindowLevel::NSNormalWindowLevel
    };
    unsafe { ns_window.setLevel(level as _) };
  }

  #[inline]
  pub fn set_window_icon(&self, _icon: Option<Icon>) {
    // macOS doesn't have window icons. Though, there is
    // `setRepresentedFilename`, but that's semantically distinct and should
    // only be used when the window is in some way representing a specific
    // file/directory. For instance, Terminal.app uses this for the CWD.
    // Anyway, that should eventually be implemented as
    // `WindowBuilderExt::with_represented_file` or something, and doesn't
    // have anything to do with `set_window_icon`.
    // https://developer.apple.com/library/content/documentation/Cocoa/Conceptual/WinPanel/Tasks/SettingWindowTitle.html
  }

  #[inline]
  pub fn set_ime_position(&self, spot: Position) {
    let ns_view = &*self.ivars().ns_view;

    let logical_spot = spot.to_logical(self.scale_factor());
    unsafe {
      view::set_ime_position(
        ns_view,
        *self.ivars().input_context,
        logical_spot.x,
        logical_spot.y,
      );
    }
  }

  #[inline]
  pub fn request_user_attention(&self, request_type: Option<UserAttentionType>) {
    let ns_request_type = request_type.map(|ty| match ty {
      UserAttentionType::Critical => NSRequestUserAttentionType::CriticalRequest,
      UserAttentionType::Informational => NSRequestUserAttentionType::InformationalRequest,
    });
    {
      let ns_window = &*self.ivars().ns_window;
      unsafe {
        let mtm = MainThreadMarker::from(ns_window.as_super());
        if let Some(ty) = ns_request_type {
          NSApp(mtm).requestUserAttention(ty);
        }
      }
    }
  }

  #[inline]
  // Allow directly accessing the current monitor internally without unwrapping.
  pub(crate) fn current_monitor_inner(&self) -> Option<RootMonitorHandle> {
    let ns_window = &*self.ivars().ns_window;
    unsafe {
      let screen: Retained<NSScreen> = ns_window.screen()?;
      let desc = NSScreen::deviceDescription(&screen);
      let key = ns_string!("NSScreenNumber");
      let value = NSDictionary::objectForKey(&desc, key).unwrap();
      let display_id = value.downcast::<NSNumber>().unwrap().unsignedIntegerValue();
      Some(RootMonitorHandle {
        inner: MonitorHandle::new(display_id.try_into().unwrap()),
      })
    }
  }

  #[inline]
  pub fn current_monitor(&self) -> Option<RootMonitorHandle> {
    self.current_monitor_inner()
  }
  #[inline]
  pub fn monitor_from_point(&self, x: f64, y: f64) -> Option<RootMonitorHandle> {
    monitor::from_point(x, y).map(|inner| RootMonitorHandle { inner })
  }

  #[inline]
  pub fn available_monitors(&self) -> VecDeque<MonitorHandle> {
    monitor::available_monitors()
  }

  #[inline]
  pub fn primary_monitor(&self) -> Option<RootMonitorHandle> {
    let monitor = monitor::primary_monitor();
    Some(RootMonitorHandle { inner: monitor })
  }

  #[inline]
  pub fn theme(&self) -> Theme {
    self.ivars().current_theme.get()
  }

  pub fn set_theme(&self, theme: Option<Theme>) {
    set_ns_theme(theme);

    self
      .ivars()
      .current_theme
      .set(theme.unwrap_or_else(get_ns_theme));
  }

  pub fn set_content_protection(&self, enabled: bool) {
    let ns_window = &*self.ivars().ns_window;
    unsafe {
      ns_window.setSharingType(if enabled {
        NSWindowSharingType::None
      } else {
        NSWindowSharingType::ReadOnly
      });
    }
  }

  pub fn set_visible_on_all_workspaces(&self, visible: bool) {
    let ns_window = &*self.ivars().ns_window;
    unsafe {
      let mut collection_behavior = ns_window.collectionBehavior();
      if visible {
        collection_behavior |= NSWindowCollectionBehavior::CanJoinAllSpaces;
      } else {
        collection_behavior &= !NSWindowCollectionBehavior::CanJoinAllSpaces;
      };
      ns_window.setCollectionBehavior(collection_behavior)
    }
  }

  pub fn set_progress_bar(&self, progress: ProgressBarState) {
    set_progress_indicator(progress);
  }

  pub fn set_badge_label(&self, label: Option<String>) {
    set_badge_label(label);
  }
}

impl WindowExtMacOS for WindowDelegate {
  #[inline]
  fn ns_window(&self) -> *mut c_void {
    self.ivars().ns_window.as_super() as *const NSWindow as *mut c_void
  }

  #[inline]
  fn ns_view(&self) -> *mut c_void {
    &*self.ivars().ns_window.contentView().unwrap() as *const NSView as *mut c_void
  }

  #[inline]
  fn simple_fullscreen(&self) -> bool {
    self.ivars().is_simple_fullscreen.get()
  }

  #[inline]
  fn set_simple_fullscreen(&self, fullscreen: bool) -> bool {
    let ns_window = &*self.ivars().ns_window;

    unsafe {
      let mtm = MainThreadMarker::from(ns_window.as_super());
      let app = NSApp(mtm);
      let is_native_fullscreen = self.ivars().fullscreen.borrow().is_some();
      let is_simple_fullscreen = self.ivars().is_simple_fullscreen.get();

      // Do nothing if native fullscreen is active.
      if is_native_fullscreen
        || (fullscreen && is_simple_fullscreen)
        || (!fullscreen && !is_simple_fullscreen)
      {
        return false;
      }

      if fullscreen {
        // Remember the original window's settings
        // Exclude title bar
        self
          .ivars()
          .standard_frame
          .set(Some(NSWindow::contentRectForFrameRect(
            ns_window,
            NSWindow::frame(ns_window),
          )));
        self.ivars().saved_style.set(Some(ns_window.styleMask()));
        self
          .ivars()
          .save_presentation_opts
          .set(Some(app.presentationOptions()));

        // Tell our window's state that we're in fullscreen
        self.ivars().is_simple_fullscreen.set(true);

        // Simulate pre-Lion fullscreen by hiding the dock and menu bar
        let presentation_options = NSApplicationPresentationOptions::AutoHideDock
          | NSApplicationPresentationOptions::AutoHideMenuBar;
        app.setPresentationOptions(presentation_options);

        // Hide the titlebar
        util::toggle_style_mask(
          ns_window,
          &self.ivars().ns_view,
          NSWindowStyleMask::Titled,
          false,
        );

        // Set the window frame to the screen frame size
        let screen = ns_window.screen().unwrap();
        let screen_frame = NSScreen::frame(&screen);
        NSWindow::setFrame_display(ns_window, screen_frame, true);

        // Fullscreen windows can't be resized, minimized, or moved
        util::toggle_style_mask(
          ns_window,
          &self.ivars().ns_view,
          NSWindowStyleMask::Miniaturizable,
          false,
        );
        util::toggle_style_mask(
          ns_window,
          &self.ivars().ns_view,
          NSWindowStyleMask::Resizable,
          false,
        );
        NSWindow::setMovable(ns_window, false);

        true
      } else {
        let new_mask = self.saved_style();
        self.ivars().is_simple_fullscreen.set(false);

        let presentation_opts = self.ivars().save_presentation_opts.get();
        let frame = self
          .ivars()
          .standard_frame
          .get()
          .unwrap_or(DEFAULT_STANDARD_FRAME);

        if let Some(presentation_opts) = presentation_opts {
          app.setPresentationOptions(presentation_opts);
        }

        ns_window.setFrame_display(frame, true);
        ns_window.setMovable(true);
        self.set_style_mask(new_mask);

        true
      }
    }
  }

  #[inline]
  fn has_shadow(&self) -> bool {
    let ns_window = &*self.ivars().ns_window;
    ns_window.hasShadow()
  }

  #[inline]
  fn set_has_shadow(&self, has_shadow: bool) {
    let ns_window = &*self.ivars().ns_window;
    ns_window.setHasShadow(has_shadow)
  }

  #[inline]
  fn set_traffic_light_inset<P: Into<Position>>(&self, position: P) {
    let position: Position = position.into();
    {
      let ns_view = &*self.ivars().ns_view;

      ns_view
        .ivars()
        .traffic_light_inset
        .set(Some(position.to_logical(self.scale_factor())));
    };
  }

  #[inline]
  fn set_is_document_edited(&self, edited: bool) {
    let ns_window = &*self.ivars().ns_window;
    ns_window.setDocumentEdited(edited)
  }

  #[inline]
  fn is_document_edited(&self) -> bool {
    let ns_window = &*self.ivars().ns_window;
    unsafe { ns_window.isDocumentEdited() }
  }

  #[inline]
  fn set_allows_automatic_window_tabbing(&self, enabled: bool) {
    let ns_window = &*self.ivars().ns_window;

    let mtm = MainThreadMarker::from(ns_window.as_super());
    NSWindow::setAllowsAutomaticWindowTabbing(enabled, mtm)
  }

  #[inline]
  fn allows_automatic_window_tabbing(&self) -> bool {
    let ns_window = &*self.ivars().ns_window;

    let mtm = MainThreadMarker::from(ns_window.as_super());
    NSWindow::allowsAutomaticWindowTabbing(mtm)
  }

  #[inline]
  fn set_tabbing_identifier(&self, identifier: &str) {
    let ns_window = &*self.ivars().ns_window;
    unsafe {
      ns_window.setTabbingIdentifier(&NSString::from_str(identifier));
    }
  }

  #[inline]
  fn tabbing_identifier(&self) -> String {
    let ns_window = &*self.ivars().ns_window;

    let tabbing_identifier = NSWindow::tabbingIdentifier(ns_window);
    tabbing_identifier.to_string()
  }

  #[inline]
  fn set_fullsize_content_view(&self, fullsize: bool) {
    let ns_window = &*self.ivars().ns_window;

    let mut mask = ns_window.styleMask();
    if fullsize {
      mask |= NSWindowStyleMask::FullSizeContentView;
    } else {
      mask &= !NSWindowStyleMask::FullSizeContentView;
    }
    self.set_style_mask(mask);
  }

  #[inline]
  fn set_titlebar_transparent(&self, transparent: bool) {
    let ns_window = &*self.ivars().ns_window;

    ns_window.setTitlebarAppearsTransparent(transparent);
  }

  fn set_badge_label(&self, label: Option<String>) {
    set_badge_label(label);
  }
}

impl WindowDelegate {
  fn content_view(&self) -> Retained<NSView> {
    self.ivars().ns_window.contentView().unwrap()
  }
  pub fn emit_event(&self, event: WindowEvent<'static>) {
    let event = Event::WindowEvent {
      window_id: WindowId(get_window_id(&self.ivars().ns_window)),
      event,
    };
    AppState::queue_event(EventWrapper::StaticEvent(event));
  }

  pub fn emit_static_scale_factor_changed_event(&self) {
    let scale_factor = self.get_scale_factor();
    if (scale_factor - self.ivars().previous_scale_factor.get()).abs() < f64::EPSILON {
      return;
    };

    self.ivars().previous_scale_factor.set(scale_factor);
    let wrapper = EventWrapper::EventProxy(EventProxy::DpiChangedProxy {
      ns_window: self.ivars().ns_window.clone().into_super(),
      suggested_size: self.view_size(),
      scale_factor,
    });
    AppState::queue_event(wrapper);
  }

  pub fn emit_resize_event(&self) {
    let rect = NSView::frame(&self.content_view());
    let scale_factor = self.get_scale_factor();
    let logical_size = LogicalSize::new(rect.size.width, rect.size.height);
    let size = logical_size.to_physical(scale_factor);
    self.emit_event(WindowEvent::Resized(size));
  }

  fn emit_move_event(&self) {
    let rect = NSWindow::frame(&self.ivars().ns_window);
    let x = rect.origin.x;
    let y = util::bottom_left_to_top_left(rect);
    let moved = self.ivars().previous_position.get() != Some((x, y));
    if moved {
      self.ivars().previous_position.set(Some((x, y)));
      let scale_factor = self.get_scale_factor();
      let physical_pos = LogicalPosition::<f64>::from((x, y)).to_physical(scale_factor);
      self.emit_event(WindowEvent::Moved(physical_pos));
    }
  }

  fn get_scale_factor(&self) -> f64 {
    NSWindow::backingScaleFactor(&self.ivars().ns_window)
  }

  fn view_size(&self) -> LogicalSize<f64> {
    let ns_size = NSView::frame(&self.content_view()).size;
    LogicalSize::new(ns_size.width, ns_size.height)
  }
}

/// Collects the file paths of a dragging pasteboard, empty if it carries none.
unsafe fn collect_paths(sender: &ProtocolObject<dyn NSDraggingInfo>) -> Vec<PathBuf> {
  let pb = sender.draggingPasteboard();
  let Some(filenames) = NSPasteboard::propertyListForType(&pb, appkit::NSFilenamesPboardType)
  else {
    return Vec::new();
  };
  let Ok(filenames) = filenames.downcast::<NSArray>() else {
    return Vec::new();
  };

  filenames
    .iter()
    .filter_map(|file| file.downcast::<NSString>().ok())
    .map(|file| {
      let path = CStr::from_ptr(NSString::UTF8String(&file))
        .to_string_lossy()
        .into_owned();
      PathBuf::from(path)
    })
    .collect()
}

define_class!(
  #[unsafe(super(NSObject))]
  #[thread_kind = MainThreadOnly]
  #[name = "TaoWindowDelegate"]
  #[ivars = State]
  pub(super) struct WindowDelegate;

  unsafe impl NSObjectProtocol for WindowDelegate {}

  unsafe impl NSWindowDelegate for WindowDelegate {
    #[unsafe(method(windowShouldClose:))]
    fn window_should_close(&self, _: Option<&AnyObject>) -> bool {
      trace!("Triggered `windowShouldClose:`");
      self.emit_event(WindowEvent::CloseRequested);
      trace!("Completed `windowShouldClose:`");
      false
    }

    #[unsafe(method(windowWillClose:))]
    fn window_will_close(&self, _: Option<&AnyObject>) {
      trace!("Triggered `windowWillClose:`");
      let state = self.ivars();
      // `setDelegate:` retains the previous value and then autoreleases it.
      // Since El Capitan, prevent delegate callbacks after the window closes.
      autoreleasepool(|_| state.ns_window.setDelegate(None));
      self.emit_event(WindowEvent::Destroyed);
      trace!("Completed `windowWillClose:`");
    }

    #[unsafe(method(windowDidResize:))]
    fn window_did_resize(&self, _: Option<&AnyObject>) {
      trace!("Triggered `windowDidResize:`");
      let state = self.ivars();
      if !state.is_checking_zoomed_in.get() {
        self.emit_resize_event();
        self.emit_move_event();
      }
      trace!("Completed `windowDidResize:`");
    }

    // This won't be triggered if the move was part of a resize.
    #[unsafe(method(windowDidMove:))]
    fn window_did_move(&self, _: Option<&AnyObject>) {
      trace!("Triggered `windowDidMove:`");
      self.emit_move_event();
      trace!("Completed `windowDidMove:`");
    }

    #[unsafe(method(windowDidChangeBackingProperties:))]
    fn window_did_change_backing_properties(&self, _: Option<&AnyObject>) {
      trace!("Triggered `windowDidChangeBackingProperties:`");
      self.emit_static_scale_factor_changed_event();
      trace!("Completed `windowDidChangeBackingProperties:`");
    }

    #[unsafe(method(windowDidBecomeKey:))]
    fn window_did_become_key(&self, _: Option<&AnyObject>) {
      trace!("Triggered `windowDidBecomeKey:`");
      // TODO: center the cursor if the window had mouse grab when it
      // lost focus
      self.emit_event(WindowEvent::Focused(true));
      trace!("Completed `windowDidBecomeKey:`");
    }

    #[unsafe(method(windowDidResignKey:))]
    fn window_did_resign_key(&self, _: Option<&AnyObject>) {
      trace!("Triggered `windowDidResignKey:`");
      let state = self.ivars();
      // It happens rather often, e.g. when the user is Cmd+Tabbing, that the
      // NSWindowDelegate will receive a didResignKey event despite no event
      // being received when the modifiers are released.  This is because
      // flagsChanged events are received by the NSView instead of the
      // NSWindowDelegate, and as a result a tracked modifiers state can quite
      // easily fall out of synchrony with reality.  This requires us to emit
      // a synthetic ModifiersChanged event when we lose focus.
      //
      let view_state = state.ns_view.ivars();

      // Both update the state and emit a ModifiersChanged event.
      if !view_state.modifiers.get().is_empty() {
        view_state.modifiers.set(ModifiersState::empty());
        self.emit_event(WindowEvent::ModifiersChanged(view_state.modifiers.get()));
      }

      self.emit_event(WindowEvent::Focused(false));
      trace!("Completed `windowDidResignKey:`");
    }

    /// Invoked before entering fullscreen.
    #[unsafe(method(windowWillEnterFullScreen:))]
    fn window_will_enter_fullscreen(&self, _: Option<&AnyObject>) {
      trace!("Triggered `windowWillEnterFullscreen:`");

      let state = self.ivars();
      state.maximized.set(self.is_zoomed());
      let fullscreen = state.fullscreen.borrow().clone();
      match fullscreen {
        // Exclusive mode sets the state in `set_fullscreen` as the user
        // can't enter exclusive mode by other means (like the
        // fullscreen button on the window decorations)
        Some(Fullscreen::Exclusive(_)) => (),
        // `window_will_enter_fullscreen` was triggered and we're already
        // in fullscreen, so we must've reached here by `set_fullscreen`
        // as it updates the state
        Some(Fullscreen::Borderless(_)) => (),
        // Otherwise, we must've reached fullscreen by the user clicking
        // on the green fullscreen button. Update state!
        None => {
          let current_monitor = self.current_monitor_inner();
          state.fullscreen.replace(Some(Fullscreen::Borderless(current_monitor)));
        }
      }
      state.in_fullscreen_transition.set(true);
      trace!("Completed `windowWillEnterFullscreen:`");
    }

    /// Invoked before exiting fullscreen.
    #[unsafe(method(windowWillExitFullScreen:))]
    fn window_will_exit_fullscreen(&self, _: Option<&AnyObject>) {
      trace!("Triggered `windowWillExitFullScreen:`");

      let state = self.ivars();
      state.in_fullscreen_transition.set(true);
      trace!("Completed `windowWillExitFullScreen:`");
    }

    #[unsafe(method(window:willUseFullScreenPresentationOptions:))]
    fn window_will_use_fullscreen_presentation_options(
      &self,
      _: Option<&AnyObject>,
      proposed_options: NSApplicationPresentationOptions,
    ) -> NSApplicationPresentationOptions {
      // Generally, games will want to disable the menu bar and the dock. Ideally,
      // this would be configurable by the user. Unfortunately because of our
      // `CGShieldingWindowLevel() + 1` hack (see `set_fullscreen`), our window is
      // placed on top of the menu bar in exclusive fullscreen mode. This looks
      // broken so we always disable the menu bar in exclusive fullscreen. We may
      // still want to make this configurable for borderless fullscreen. Right now
      // we don't, for consistency. If we do, it should be documented that the
      // user-provided options are ignored in exclusive fullscreen.
      let mut options = proposed_options;
      let state = self.ivars();
      if let Some(Fullscreen::Exclusive(_)) = *state.fullscreen.borrow() {
        options = NSApplicationPresentationOptions::FullScreen
          | NSApplicationPresentationOptions::HideDock
          | NSApplicationPresentationOptions::HideMenuBar;
      }

      options
    }

    /// Invoked after entering fullscreen.
    #[unsafe(method(windowDidEnterFullScreen:))]
    fn window_did_enter_fullscreen(&self, _: Option<&AnyObject>) {
      trace!("Triggered `windowDidEnterFullscreen:`");
      let state = self.ivars();
      state.initial_fullscreen.set(false);
      state.in_fullscreen_transition.set(false);
      let target_fullscreen = state.target_fullscreen.borrow_mut().take();

      if let Some(target_fullscreen) = target_fullscreen {
        self.set_fullscreen(target_fullscreen);
      }
      self.emit_resize_event();
      self.emit_move_event();
      trace!("Completed `windowDidEnterFullscreen:`");
    }

    /// Invoked after exiting fullscreen.
    #[unsafe(method(windowDidExitFullScreen:))]
    fn window_did_exit_fullscreen(&self, _: Option<&AnyObject>) {
      trace!("Triggered `windowDidExitFullscreen:`");
      let state = self.ivars();
      self.restore_state_from_fullscreen();

      state.in_fullscreen_transition.set(false);
      let target_fullscreen = state.target_fullscreen.borrow_mut().take();

      if let Some(target_fullscreen) = target_fullscreen {
        self.set_fullscreen(target_fullscreen);
      }
      // Leaving fullscreen resets the traffic light buttons to their default
      // position without a `drawRect:`, so re-apply the custom inset (#15451).
      unsafe { reapply_traffic_light_inset(&state.ns_window, &state.ns_view) };
      self.emit_resize_event();
      self.emit_move_event();
      trace!("Completed `windowDidExitFullscreen:`");
    }

    /// Invoked when entering fullscreen fails.
    ///
    /// When this window launch from a fullscreen app (e.g. launch from VS Code
    /// terminal), it creates a new virtual destkop and a transition animation.
    /// This animation takes one second and cannot be disable without
    /// elevated privileges. In this animation time, all toggleFullscreen events
    /// will be failed. In this implementation, we will try again by using
    /// performSelector:withObject:afterDelay: until window_did_enter_fullscreen.
    /// It should be fine as we only do this at initialzation (i.e with_fullscreen
    /// was set).
    ///
    /// From Apple doc:
    /// In some cases, the transition to enter full-screen mode can fail,
    /// due to being in the midst of handling some other animation or user gesture.
    /// This method indicates that there was an error, and you should clean up any
    /// work you may have done to prepare to enter full-screen mode.

    #[unsafe(method(windowDidFailToEnterFullScreen:))]
    fn window_did_fail_to_enter_fullscreen(&self, _: Option<&AnyObject>) {
      trace!("Triggered `windowDidFailToEnterFullscreen:`");
      let state = self.ivars();
      state.in_fullscreen_transition.set(false);
      state.target_fullscreen.replace(None);
      if state.initial_fullscreen.get() {
          unsafe {
            state.ns_window.performSelector_withObject_afterDelay(
              sel!(toggleFullScreen:),
              None,
              0.5,
            )
          }
      } else {
        self.restore_state_from_fullscreen();
      }
      trace!("Completed `windowDidFailToEnterFullscreen:`");
    }
  }

  unsafe impl NSDraggingDestination for WindowDelegate {
    /// Invoked when the dragged image enters destination bounds or frame.
    #[unsafe(method(draggingEntered:))]
    fn dragging_entered(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
      trace!("Triggered `draggingEntered:`");

      for path in unsafe { collect_paths(sender) } {
        self.emit_event(WindowEvent::HoveredFile(path));
      }

      trace!("Completed `draggingEntered:`");
      NSDragOperation::Copy
    }

    /// Invoked when the image is released.
    #[unsafe(method(prepareForDragOperation:))]
    fn prepare_for_drag_operation(&self, _: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
      trace!("Triggered `prepareForDragOperation:`");
      trace!("Completed `prepareForDragOperation:`");
      true
    }

    /// Invoked after the released image has been removed from the screen.
    #[unsafe(method(performDragOperation:))]
    fn perform_drag_operation(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
      trace!("Triggered `performDragOperation:`");

      for path in unsafe { collect_paths(sender) } {
        self.emit_event(WindowEvent::DroppedFile(path));
      }

      trace!("Completed `performDragOperation:`");
      true
    }

    /// Invoked when the dragging operation is complete.
    #[unsafe(method(concludeDragOperation:))]
    fn conclude_drag_operation(&self, _: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
      trace!("Triggered `concludeDragOperation:`");
      trace!("Completed `concludeDragOperation:`");
    }

    /// Invoked when the dragging operation is cancelled.
    #[unsafe(method(draggingExited:))]
    fn dragging_exited(&self, _: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
      trace!("Triggered `draggingExited:`");
      self.emit_event(WindowEvent::HoveredFileCancelled);
      trace!("Completed `draggingExited:`");
    }
  }

  impl WindowDelegate {
    #[unsafe(method(markIsCheckingZoomedIn))]
    fn mark_is_checking_zoomed_in(&self) {
      let state = self.ivars();
      state.is_checking_zoomed_in.set(true);
    }

    #[unsafe(method(clearIsCheckingZoomedIn))]
    fn clear_is_checking_zoomed_in(&self) {
      let state = self.ivars();
      state.is_checking_zoomed_in.set(false);
    }

    // Observe theme changes.
    #[unsafe(method(effectiveAppearanceDidChange:))]
    fn effective_appearance_did_change(&self, _: Option<&AnyObject>) {
      trace!("Triggered `effectiveAppearDidChange:`");
      unsafe { self.performSelectorOnMainThread_withObject_waitUntilDone(sel!(effectiveAppearanceDidChangedOnMainThread:), None, false) };
    }

    #[unsafe(method(effectiveAppearanceDidChangedOnMainThread:))]
    fn effective_appearance_did_changed_on_main_thread(&self, _: Option<&AnyObject>) {
      let state = self.ivars();
      let theme = get_ns_theme();
      let current_theme = state.current_theme.replace(theme);
      if current_theme != theme {
        self.emit_event(WindowEvent::ThemeChanged(theme));
      }
      trace!("Completed `effectiveAppearDidChange:`");
    }
  }
);

unsafe fn set_min_inner_size(window: &NSWindow, mut min_size: LogicalSize<f64>) {
  let mut current_rect = NSWindow::frame(window);
  let content_rect = NSWindow::contentRectForFrameRect(window, NSWindow::frame(window));
  // Convert from client area size to window size
  min_size.width += current_rect.size.width - content_rect.size.width; // this tends to be 0
  min_size.height += current_rect.size.height - content_rect.size.height;
  window.setMinSize(NSSize {
    width: min_size.width as CGFloat,
    height: min_size.height as CGFloat,
  });
  // If necessary, resize the window to match constraint
  if current_rect.size.width < min_size.width {
    current_rect.size.width = min_size.width;
    window.setFrame_display(current_rect, false)
  }
  if current_rect.size.height < min_size.height {
    // The origin point of a rectangle is at its bottom left in Cocoa.
    // To ensure the window's top-left point remains the same:
    current_rect.origin.y += current_rect.size.height - min_size.height;
    current_rect.size.height = min_size.height;
    window.setFrame_display(current_rect, false)
  }
}

unsafe fn set_max_inner_size(window: &NSWindow, mut max_size: LogicalSize<f64>) {
  let mut current_rect = NSWindow::frame(window);
  let content_rect = NSWindow::contentRectForFrameRect(window, NSWindow::frame(window));
  // Convert from client area size to window size
  max_size.width += current_rect.size.width - content_rect.size.width; // this tends to be 0
  max_size.height += current_rect.size.height - content_rect.size.height;
  window.setMaxSize(NSSize {
    width: max_size.width as CGFloat,
    height: max_size.height as CGFloat,
  });
  // If necessary, resize the window to match constraint
  if current_rect.size.width > max_size.width {
    current_rect.size.width = max_size.width;
    window.setFrame_display(current_rect, false)
  }
  if current_rect.size.height > max_size.height {
    // The origin point of a rectangle is at its bottom left in Cocoa.
    // To ensure the window's top-left point remains the same:
    current_rect.origin.y += current_rect.size.height - max_size.height;
    current_rect.size.height = max_size.height;
    window.setFrame_display(current_rect, false)
  }
}

const DEFAULT_STANDARD_FRAME: NSRect =
  NSRect::new(NSPoint::new(50.0, 50.0), NSSize::new(800.0, 600.0));
