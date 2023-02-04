// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  cell::RefCell,
  collections::VecDeque,
  rc::Rc,
  sync::{
    atomic::{AtomicBool, AtomicI32, Ordering},
    Arc,
  },
};

use gdk::{WindowEdge, WindowState};
use glib::translate::ToGlibPtr;
use gtk::{prelude::*, traits::SettingsExt, AccelGroup, Orientation, Settings};
use raw_window_handle::{
  RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, XlibDisplayHandle,
  XlibWindowHandle,
};

use crate::{
  dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position, Size},
  error::{ExternalError, NotSupportedError, OsError as RootOsError},
  icon::Icon,
  menu::{MenuId, MenuItem},
  monitor::MonitorHandle as RootMonitorHandle,
  window::{
    CursorIcon, Fullscreen, Theme, UserAttentionType, WindowAttributes, BORDERLESS_RESIZE_INSET,
  },
};

use super::{
  event_loop::EventLoopWindowTarget,
  menu,
  monitor::{self, MonitorHandle},
  util, Parent, PlatformSpecificWindowBuilderAttributes,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(pub(crate) u32);

impl WindowId {
  pub fn dummy() -> Self {
    WindowId(u32::MAX)
  }
}

// Currently GTK doesn't provide feature for detect theme, so we need to check theme manually.
// ref: https://github.com/WebKit/WebKit/blob/e44ffaa0d999a9807f76f1805943eea204cfdfbc/Source/WebKit/UIProcess/API/gtk/PageClientImpl.cpp#L587
const GTK_THEME_SUFFIX_LIST: [&'static str; 3] = ["-dark", "-Dark", "-Darker"];

pub struct Window {
  /// Window id.
  pub(crate) window_id: WindowId,
  /// Gtk application window.
  pub(crate) window: gtk::ApplicationWindow,
  /// Window requests sender
  pub(crate) window_requests_tx: glib::Sender<(WindowId, WindowRequest)>,
  /// Gtk Acceleration Group
  pub(crate) accel_group: AccelGroup,
  // Gtk MenuBar allocation -- always available
  menu_bar: gtk::MenuBar,
  scale_factor: Rc<AtomicI32>,
  position: Rc<(AtomicI32, AtomicI32)>,
  size: Rc<(AtomicI32, AtomicI32)>,
  maximized: Rc<AtomicBool>,
  minimized: Rc<AtomicBool>,
  fullscreen: RefCell<Option<Fullscreen>>,
  min_inner_size: RefCell<Option<Size>>,
  max_inner_size: RefCell<Option<Size>>,
  /// Draw event Sender
  draw_tx: crossbeam_channel::Sender<WindowId>,
}

impl Window {
  pub(crate) fn new<T>(
    event_loop_window_target: &EventLoopWindowTarget<T>,
    attributes: WindowAttributes,
    pl_attribs: PlatformSpecificWindowBuilderAttributes,
  ) -> Result<Self, RootOsError> {
    let app = &event_loop_window_target.app;
    let window_requests_tx = event_loop_window_target.window_requests_tx.clone();
    let draw_tx = event_loop_window_target.draw_tx.clone();
    let window = gtk::ApplicationWindow::builder()
      .application(app)
      .accept_focus(attributes.focused)
      .build();
    let window_id = WindowId(window.id());
    event_loop_window_target
      .windows
      .borrow_mut()
      .insert(window_id);

    let accel_group = AccelGroup::new();
    window.add_accel_group(&accel_group);

    // Set Width/Height & Resizable
    let win_scale_factor = window.scale_factor();
    let (width, height) = attributes
      .inner_size
      .map(|size| size.to_logical::<f64>(win_scale_factor as f64).into())
      .unwrap_or((800, 600));
    window.set_default_size(1, 1);
    window.resize(width, height);

    if attributes.maximized {
      window.maximize();
    }

    window.set_resizable(attributes.resizable);
    window.set_deletable(attributes.closable);

    // Set Min/Max Size
    let geom_mask = attributes
      .min_inner_size
      .map(|_| gdk::WindowHints::MIN_SIZE)
      .unwrap_or(gdk::WindowHints::empty())
      | attributes
        .max_inner_size
        .map(|_| gdk::WindowHints::MAX_SIZE)
        .unwrap_or(gdk::WindowHints::empty());
    let (min_width, min_height) = attributes
      .min_inner_size
      .map(|size| size.to_logical::<f64>(win_scale_factor as f64).into())
      .unwrap_or_default();
    let (max_width, max_height) = attributes
      .max_inner_size
      .map(|size| size.to_logical::<f64>(win_scale_factor as f64).into())
      .unwrap_or_default();
    let picky_none: Option<&gtk::Window> = None;
    window.set_geometry_hints(
      picky_none,
      Some(&gdk::Geometry::new(
        min_width,
        min_height,
        max_width,
        max_height,
        0,
        0,
        0,
        0,
        0f64,
        0f64,
        gdk::Gravity::Center,
      )),
      geom_mask,
    );

    // Set Position
    if let Some(position) = attributes.position {
      let (x, y): (i32, i32) = position.to_logical::<i32>(win_scale_factor as f64).into();
      window.move_(x, y);
    }

    // Set GDK Visual
    if pl_attribs.rgba_visual || attributes.transparent {
      if let Some(screen) = GtkWindowExt::screen(&window) {
        if let Some(visual) = screen.rgba_visual() {
          window.set_visual(Some(&visual));
        }
      }
    }

    if pl_attribs.app_paintable || attributes.transparent {
      // Set a few attributes to make the window can be painted.
      // See Gtk drawing model for more info:
      // https://docs.gtk.org/gtk3/drawing-model.html
      window.set_app_paintable(true);
    }

    if !pl_attribs.double_buffered {
      let widget = window.upcast_ref::<gtk::Widget>();
      if !event_loop_window_target.is_wayland() {
        unsafe {
          gtk::ffi::gtk_widget_set_double_buffered(widget.to_glib_none().0, 0);
        }
      }
    }

    // We always create a box and allocate menubar, so if they set_menu after creation
    // we can inject the menubar without re-redendering the whole window
    let window_box = gtk::Box::new(Orientation::Vertical, 0);
    window.add(&window_box);

    let mut menu_bar = gtk::MenuBar::new();

    if attributes.transparent {
      let style_context = menu_bar.style_context();
      let css_provider = gtk::CssProvider::new();
      let theme = r#"
          menubar {
            background-color: transparent;
            box-shadow: none;
          }
        "#;
      let _ = css_provider.load_from_data(theme.as_bytes());
      style_context.add_provider(&css_provider, 600);
    }
    window_box.pack_start(&menu_bar, false, false, 0);
    if let Some(window_menu) = attributes.window_menu {
      window_menu.generate_menu(&mut menu_bar, &window_requests_tx, &accel_group, window_id);
    }

    // Rest attributes
    window.set_title(&attributes.title);
    if let Some(Fullscreen::Borderless(m)) = &attributes.fullscreen {
      if let Some(monitor) = m {
        let display = window.display();
        let monitor = &monitor.inner;
        let monitors = display.n_monitors();
        for i in 0..monitors {
          let m = display.monitor(i).unwrap();
          if m == monitor.monitor {
            let screen = display.default_screen();
            window.fullscreen_on_monitor(&screen, i);
          }
        }
      } else {
        window.fullscreen();
      }
    }
    window.set_visible(attributes.visible);
    window.set_decorated(attributes.decorations);

    if attributes.always_on_bottom {
      window.set_keep_below(attributes.always_on_bottom);
    }

    if attributes.always_on_top {
      window.set_keep_above(attributes.always_on_top);
    }

    if attributes.visible_on_all_workspaces {
      window.stick();
    }

    if let Some(icon) = attributes.window_icon {
      window.set_icon(Some(&icon.inner.into()));
    }

    let settings = Settings::default();

    if let Some(settings) = settings {
      if let Some(preferred_theme) = attributes.preferred_theme {
        match preferred_theme {
          Theme::Dark => settings.set_gtk_application_prefer_dark_theme(true),
          Theme::Light => {
            let theme_name = settings.gtk_theme_name().map(|t| t.as_str().to_owned());
            if let Some(theme) = theme_name {
              // Remove dark variant.
              if let Some(theme) = GTK_THEME_SUFFIX_LIST
                .iter()
                .find(|t| theme.ends_with(*t))
                .map(|v| theme.strip_suffix(v))
              {
                settings.set_gtk_theme_name(theme);
              }
            }
          }
        }
      }
    }

    if attributes.visible {
      window.show_all();
    } else {
      window.hide();
    }

    if let Parent::ChildOf(parent) = pl_attribs.parent {
      window.set_transient_for(Some(&parent));
    }

    // restore accept-focus after the window has been drawn
    // if the window was initially created without focus
    if !attributes.focused {
      let signal_id = Arc::new(RefCell::new(None));
      let signal_id_ = signal_id.clone();
      let id = window.connect_draw(move |window, _| {
        if let Some(id) = signal_id_.take() {
          window.set_accept_focus(true);
          window.disconnect(id);
        }
        Inhibit(false)
      });
      signal_id.borrow_mut().replace(id);
    }

    let w_pos = window.position();
    let position: Rc<(AtomicI32, AtomicI32)> = Rc::new((w_pos.0.into(), w_pos.1.into()));
    let position_clone = position.clone();

    let w_size = window.size();
    let size: Rc<(AtomicI32, AtomicI32)> = Rc::new((w_size.0.into(), w_size.1.into()));
    let size_clone = size.clone();

    window.connect_configure_event(move |_, event| {
      let (x, y) = event.position();
      position_clone.0.store(x, Ordering::Release);
      position_clone.1.store(y, Ordering::Release);

      let (w, h) = event.size();
      size_clone.0.store(w as i32, Ordering::Release);
      size_clone.1.store(h as i32, Ordering::Release);

      false
    });

    let w_max = window.is_maximized();
    let maximized: Rc<AtomicBool> = Rc::new(w_max.into());
    let max_clone = maximized.clone();
    let minimized = Rc::new(AtomicBool::new(false));
    let min_clone = minimized.clone();

    window.connect_window_state_event(move |_, event| {
      let state = event.new_window_state();
      max_clone.store(state.contains(WindowState::MAXIMIZED), Ordering::Release);
      min_clone.store(state.contains(WindowState::ICONIFIED), Ordering::Release);
      Inhibit(false)
    });

    let scale_factor: Rc<AtomicI32> = Rc::new(win_scale_factor.into());
    let scale_factor_clone = scale_factor.clone();
    window.connect_scale_factor_notify(move |window| {
      scale_factor_clone.store(window.scale_factor(), Ordering::Release);
    });

    // Check if we should paint the transparent background ourselves.
    let mut transparent = false;
    if attributes.transparent && pl_attribs.auto_transparent {
      transparent = true;
    }
    let cursor_moved = pl_attribs.cursor_moved;
    if let Err(e) = window_requests_tx.send((
      window_id,
      WindowRequest::WireUpEvents {
        transparent,
        cursor_moved,
      },
    )) {
      log::warn!("Fail to send wire up events request: {}", e);
    }

    if let Err(e) = draw_tx.send(window_id) {
      log::warn!("Failed to send redraw event to event channel: {}", e);
    }

    let win = Self {
      window_id,
      window,
      window_requests_tx,
      draw_tx,
      accel_group,
      menu_bar,
      scale_factor,
      position,
      size,
      maximized,
      minimized,
      fullscreen: RefCell::new(attributes.fullscreen),
      min_inner_size: RefCell::new(attributes.min_inner_size),
      max_inner_size: RefCell::new(attributes.max_inner_size),
    };

    win.set_skip_taskbar(pl_attribs.skip_taskbar);

    Ok(win)
  }

  pub fn id(&self) -> WindowId {
    self.window_id
  }

  pub fn scale_factor(&self) -> f64 {
    self.scale_factor.load(Ordering::Acquire) as f64
  }

  pub fn request_redraw(&self) {
    if let Err(e) = self.draw_tx.send(self.window_id) {
      log::warn!("Failed to send redraw event to event channel: {}", e);
    }
  }

  pub fn inner_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError> {
    let (x, y) = &*self.position;
    Ok(
      LogicalPosition::new(x.load(Ordering::Acquire), y.load(Ordering::Acquire))
        .to_physical(self.scale_factor.load(Ordering::Acquire) as f64),
    )
  }

  pub fn outer_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError> {
    let (x, y) = &*self.position;
    Ok(
      LogicalPosition::new(x.load(Ordering::Acquire), y.load(Ordering::Acquire))
        .to_physical(self.scale_factor.load(Ordering::Acquire) as f64),
    )
  }

  pub fn set_outer_position<P: Into<Position>>(&self, position: P) {
    let (x, y): (i32, i32) = position
      .into()
      .to_logical::<i32>(self.scale_factor())
      .into();

    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::Position((x, y))))
    {
      log::warn!("Fail to send position request: {}", e);
    }
  }

  pub fn inner_size(&self) -> PhysicalSize<u32> {
    let (width, height) = &*self.size;

    LogicalSize::new(
      width.load(Ordering::Acquire) as u32,
      height.load(Ordering::Acquire) as u32,
    )
    .to_physical(self.scale_factor.load(Ordering::Acquire) as f64)
  }

  pub fn set_inner_size<S: Into<Size>>(&self, size: S) {
    let (width, height) = size.into().to_logical::<i32>(self.scale_factor()).into();

    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::Size((width, height))))
    {
      log::warn!("Fail to send size request: {}", e);
    }
  }

  pub fn outer_size(&self) -> PhysicalSize<u32> {
    let (width, height) = &*self.size;

    LogicalSize::new(
      width.load(Ordering::Acquire) as u32,
      height.load(Ordering::Acquire) as u32,
    )
    .to_physical(self.scale_factor.load(Ordering::Acquire) as f64)
  }

  pub fn set_min_inner_size<S: Into<Size>>(&self, min_size: Option<S>) {
    *self.min_inner_size.borrow_mut() = min_size.map(Into::into);

    let scale_factor = self.scale_factor();

    let min = self
      .min_inner_size
      .borrow()
      .map(|s| s.to_logical::<i32>(scale_factor));
    let max = self
      .max_inner_size
      .borrow()
      .map(|s| s.to_logical::<i32>(scale_factor));

    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::SizeConstraint { min, max }))
    {
      log::warn!("Fail to send min size request: {}", e);
    }
  }
  pub fn set_max_inner_size<S: Into<Size>>(&self, max_size: Option<S>) {
    *self.max_inner_size.borrow_mut() = max_size.map(Into::into);

    let scale_factor = self.scale_factor();

    let min = self
      .min_inner_size
      .borrow()
      .map(|s| s.to_logical::<i32>(scale_factor));
    let max = self
      .max_inner_size
      .borrow()
      .map(|s| s.to_logical::<i32>(scale_factor));

    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::SizeConstraint { min, max }))
    {
      log::warn!("Fail to send max size request: {}", e);
    }
  }

  pub fn set_title(&self, title: &str) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::Title(title.to_string())))
    {
      log::warn!("Fail to send title request: {}", e);
    }
  }

  pub fn title(&self) -> String {
    self
      .window
      .title()
      .map(|t| t.as_str().to_string())
      .unwrap_or_default()
  }

  pub fn set_menu(&self, menu: Option<menu::Menu>) {
    if let Err(e) = self.window_requests_tx.send((
      self.window_id,
      WindowRequest::SetMenu((menu, self.accel_group.clone(), self.menu_bar.clone())),
    )) {
      log::warn!("Fail to send menu request: {}", e);
    }
  }

  pub fn set_visible(&self, visible: bool) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::Visible(visible)))
    {
      log::warn!("Fail to send visible request: {}", e);
    }
  }

  pub fn set_focus(&self) {
    if !self.minimized.load(Ordering::Acquire) && self.window.get_visible() {
      if let Err(e) = self
        .window_requests_tx
        .send((self.window_id, WindowRequest::Focus))
      {
        log::warn!("Fail to send visible request: {}", e);
      }
    }
  }

  pub fn is_focused(&self) -> bool {
    self.window.is_active()
  }

  pub fn set_resizable(&self, resizable: bool) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::Resizable(resizable)))
    {
      log::warn!("Fail to send resizable request: {}", e);
    }
  }

  pub fn set_minimizable(&self, _minimizable: bool) {}

  pub fn set_maximizable(&self, _maximizable: bool) {}

  pub fn set_closable(&self, closable: bool) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::Closable(closable)))
    {
      log::warn!("Fail to send closable request: {}", e);
    }
  }

  pub fn set_minimized(&self, minimized: bool) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::Minimized(minimized)))
    {
      log::warn!("Fail to send minimized request: {}", e);
    }
  }

  pub fn set_maximized(&self, maximized: bool) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::Maximized(maximized)))
    {
      log::warn!("Fail to send maximized request: {}", e);
    }
  }

  pub fn is_maximized(&self) -> bool {
    self.maximized.load(Ordering::Acquire)
  }

  pub fn is_minimized(&self) -> bool {
    self.minimized.load(Ordering::Acquire)
  }

  pub fn is_resizable(&self) -> bool {
    self.window.is_resizable()
  }

  pub fn is_minimizable(&self) -> bool {
    true
  }

  pub fn is_maximizable(&self) -> bool {
    true
  }

  pub fn is_closable(&self) -> bool {
    self.window.is_deletable()
  }

  pub fn is_decorated(&self) -> bool {
    self.window.is_decorated()
  }

  #[inline]
  pub fn is_visible(&self) -> bool {
    self.window.is_visible()
  }

  pub fn drag_window(&self) -> Result<(), ExternalError> {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::DragWindow))
    {
      log::warn!("Fail to send drag window request: {}", e);
    }
    Ok(())
  }

  pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
    self.fullscreen.replace(fullscreen.clone());
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::Fullscreen(fullscreen)))
    {
      log::warn!("Fail to send fullscreen request: {}", e);
    }
  }

  pub fn fullscreen(&self) -> Option<Fullscreen> {
    self.fullscreen.borrow().clone()
  }

  pub fn set_decorations(&self, decorations: bool) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::Decorations(decorations)))
    {
      log::warn!("Fail to send decorations request: {}", e);
    }
  }

  pub fn set_always_on_bottom(&self, always_on_bottom: bool) {
    if let Err(e) = self.window_requests_tx.send((
      self.window_id,
      WindowRequest::AlwaysOnBottom(always_on_bottom),
    )) {
      log::warn!("Fail to send always on bottom request: {}", e);
    }
  }

  pub fn set_always_on_top(&self, always_on_top: bool) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::AlwaysOnTop(always_on_top)))
    {
      log::warn!("Fail to send always on top request: {}", e);
    }
  }

  pub fn set_window_icon(&self, window_icon: Option<Icon>) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::WindowIcon(window_icon)))
    {
      log::warn!("Fail to send window icon request: {}", e);
    }
  }

  pub fn set_ime_position<P: Into<Position>>(&self, _position: P) {
    //TODO
  }

  pub fn request_user_attention(&self, request_type: Option<UserAttentionType>) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::UserAttention(request_type)))
    {
      log::warn!("Fail to send user attention request: {}", e);
    }
  }

  pub fn hide_menu(&self) {
    self.menu_bar.hide();
  }

  pub fn show_menu(&self) {
    self.menu_bar.show_all();
  }

  pub fn is_menu_visible(&self) -> bool {
    self.menu_bar.get_visible()
  }

  pub fn set_visible_on_all_workspaces(&self, visible: bool) {
    if let Err(e) = self.window_requests_tx.send((
      self.window_id,
      WindowRequest::SetVisibleOnAllWorkspaces(visible),
    )) {
      log::warn!("Fail to send visible on all workspaces request: {}", e);
    }
  }

  pub fn set_cursor_icon(&self, cursor: CursorIcon) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::CursorIcon(Some(cursor))))
    {
      log::warn!("Fail to send cursor icon request: {}", e);
    }
  }

  pub fn set_cursor_position<P: Into<Position>>(&self, position: P) -> Result<(), ExternalError> {
    let inner_pos = self.inner_position().unwrap_or_default();
    let (x, y): (i32, i32) = position
      .into()
      .to_logical::<i32>(self.scale_factor())
      .into();

    if let Err(e) = self.window_requests_tx.send((
      self.window_id,
      WindowRequest::CursorPosition((x + inner_pos.x, y + inner_pos.y)),
    )) {
      log::warn!("Fail to send cursor position request: {}", e);
    }

    Ok(())
  }

  pub fn set_cursor_grab(&self, _grab: bool) -> Result<(), ExternalError> {
    Ok(())
  }

  pub fn set_ignore_cursor_events(&self, ignore: bool) -> Result<(), ExternalError> {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::CursorIgnoreEvents(ignore)))
    {
      log::warn!("Fail to send cursor position request: {}", e);
    }

    Ok(())
  }

  pub fn set_cursor_visible(&self, visible: bool) {
    let cursor = if visible {
      Some(CursorIcon::Default)
    } else {
      None
    };
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::CursorIcon(cursor)))
    {
      log::warn!("Fail to send cursor visibility request: {}", e);
    }
  }

  #[inline]
  pub fn cursor_position(&self) -> Result<PhysicalPosition<f64>, ExternalError> {
    util::cursor_position()
  }

  pub fn current_monitor(&self) -> Option<RootMonitorHandle> {
    let display = self.window.display();
    // `.window()` returns `None` if the window is invisible;
    // we fallback to the primary monitor
    let monitor = self
      .window
      .window()
      .map(|window| display.monitor_at_window(&window))
      .unwrap_or_else(|| display.primary_monitor())
      .unwrap();
    let handle = MonitorHandle { monitor };
    Some(RootMonitorHandle { inner: handle })
  }

  #[inline]
  pub fn available_monitors(&self) -> VecDeque<MonitorHandle> {
    let mut handles = VecDeque::new();
    let display = self.window.display();
    let numbers = display.n_monitors();

    for i in 0..numbers {
      let monitor = MonitorHandle::new(&display, i);
      handles.push_back(monitor);
    }

    handles
  }

  pub fn primary_monitor(&self) -> Option<RootMonitorHandle> {
    let display = self.window.display();
    let monitor = display.primary_monitor().unwrap();
    let handle = MonitorHandle { monitor };
    Some(RootMonitorHandle { inner: handle })
  }

  #[inline]
  pub fn monitor_from_point(&self, x: f64, y: f64) -> Option<RootMonitorHandle> {
    let display = &self.window.display();
    monitor::from_point(display, x, y).map(|inner| RootMonitorHandle { inner })
  }

  fn is_wayland(&self) -> bool {
    self.window.display().backend().is_wayland()
  }

  pub fn raw_window_handle(&self) -> RawWindowHandle {
    if self.is_wayland() {
      let mut window_handle = WaylandWindowHandle::empty();
      if let Some(window) = self.window.window() {
        window_handle.surface =
          unsafe { gdk_wayland_sys::gdk_wayland_window_get_wl_surface(window.as_ptr() as *mut _) };
      }

      RawWindowHandle::Wayland(window_handle)
    } else {
      let mut window_handle = XlibWindowHandle::empty();
      unsafe {
        if let Some(window) = self.window.window() {
          window_handle.window = gdk_x11_sys::gdk_x11_window_get_xid(window.as_ptr() as *mut _);
        }
      }
      RawWindowHandle::Xlib(window_handle)
    }
  }

  pub fn raw_display_handle(&self) -> RawDisplayHandle {
    if self.is_wayland() {
      let mut display_handle = WaylandDisplayHandle::empty();
      display_handle.display = unsafe {
        gdk_wayland_sys::gdk_wayland_display_get_wl_display(self.window.display().as_ptr() as *mut _)
      };
      RawDisplayHandle::Wayland(display_handle)
    } else {
      let mut display_handle = XlibDisplayHandle::empty();
      unsafe {
        if let Ok(xlib) = x11_dl::xlib::Xlib::open() {
          let display = (xlib.XOpenDisplay)(std::ptr::null());
          display_handle.display = display as _;
          display_handle.screen = (xlib.XDefaultScreen)(display) as _;
        }
      }

      RawDisplayHandle::Xlib(display_handle)
    }
  }

  pub(crate) fn set_skip_taskbar(&self, skip: bool) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::SetSkipTaskbar(skip)))
    {
      log::warn!("Fail to send skip taskbar request: {}", e);
    }
  }

  pub fn theme(&self) -> Theme {
    if let Some(settings) = Settings::default() {
      let theme_name = settings.gtk_theme_name().map(|s| s.as_str().to_owned());
      if let Some(theme) = theme_name {
        if GTK_THEME_SUFFIX_LIST.iter().any(|t| theme.ends_with(t)) {
          return Theme::Dark;
        }
      }
    }
    return Theme::Light;
  }
}

// We need GtkWindow to initialize WebView, so we have to keep it in the field.
// It is called on any method.
unsafe impl Send for Window {}
unsafe impl Sync for Window {}

#[non_exhaustive]
pub enum WindowRequest {
  Title(String),
  Position((i32, i32)),
  Size((i32, i32)),
  SizeConstraint {
    min: Option<LogicalSize<i32>>,
    max: Option<LogicalSize<i32>>,
  },
  Visible(bool),
  Focus,
  Resizable(bool),
  Closable(bool),
  Minimized(bool),
  Maximized(bool),
  DragWindow,
  Fullscreen(Option<Fullscreen>),
  Decorations(bool),
  AlwaysOnBottom(bool),
  AlwaysOnTop(bool),
  WindowIcon(Option<Icon>),
  UserAttention(Option<UserAttentionType>),
  SetSkipTaskbar(bool),
  CursorIcon(Option<CursorIcon>),
  CursorPosition((i32, i32)),
  CursorIgnoreEvents(bool),
  WireUpEvents {
    transparent: bool,
    cursor_moved: bool,
  },
  Menu((Option<MenuItem>, Option<MenuId>)),
  SetMenu((Option<menu::Menu>, AccelGroup, gtk::MenuBar)),
  GlobalHotKey(u16),
  SetVisibleOnAllWorkspaces(bool),
}

pub fn hit_test(window: &gdk::Window, cx: f64, cy: f64) -> WindowEdge {
  let (left, top) = window.position();
  let (w, h) = (window.width(), window.height());
  let (right, bottom) = (left + w, top + h);
  let (cx, cy) = (cx as i32, cy as i32);

  const LEFT: i32 = 0b0001;
  const RIGHT: i32 = 0b0010;
  const TOP: i32 = 0b0100;
  const BOTTOM: i32 = 0b1000;
  const TOPLEFT: i32 = TOP | LEFT;
  const TOPRIGHT: i32 = TOP | RIGHT;
  const BOTTOMLEFT: i32 = BOTTOM | LEFT;
  const BOTTOMRIGHT: i32 = BOTTOM | RIGHT;

  let inset = BORDERLESS_RESIZE_INSET * window.scale_factor();
  #[rustfmt::skip]
  let result =
      (LEFT * (if cx < (left + inset) { 1 } else { 0 }))
    | (RIGHT * (if cx >= (right - inset) { 1 } else { 0 }))
    | (TOP * (if cy < (top + inset) { 1 } else { 0 }))
    | (BOTTOM * (if cy >= (bottom - inset) { 1 } else { 0 }));

  match result {
    LEFT => WindowEdge::West,
    TOP => WindowEdge::North,
    RIGHT => WindowEdge::East,
    BOTTOM => WindowEdge::South,
    TOPLEFT => WindowEdge::NorthWest,
    TOPRIGHT => WindowEdge::NorthEast,
    BOTTOMLEFT => WindowEdge::SouthWest,
    BOTTOMRIGHT => WindowEdge::SouthEast,
    // we return `WindowEdge::__Unknown` to be ignored later.
    // we must return 8 or bigger, otherwise it will be the same as one of the other 7 variants of `WindowEdge` enum.
    _ => WindowEdge::__Unknown(8),
  }
}

impl Drop for Window {
  fn drop(&mut self) {
    unsafe {
      self.window.destroy();
    }
  }
}
