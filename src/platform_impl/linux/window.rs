// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  cell::RefCell,
  collections::VecDeque,
  rc::Rc,
  sync::atomic::{AtomicBool, AtomicI32, Ordering},
};

use gdk::{WindowEdge, WindowState};
use gdk_pixbuf::{Colorspace, Pixbuf};
use gtk::{prelude::*, AccelGroup, Orientation};
use raw_window_handle::{RawWindowHandle, XlibHandle};

use crate::{
  dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position, Size},
  error::{ExternalError, NotSupportedError, OsError as RootOsError},
  icon::{BadIcon, Icon},
  menu::{MenuId, MenuItem},
  monitor::MonitorHandle as RootMonitorHandle,
  window::{CursorIcon, Fullscreen, UserAttentionType, WindowAttributes, BORDERLESS_RESIZE_INSET},
};

use super::{
  event_loop::EventLoopWindowTarget, menu, monitor::MonitorHandle,
  PlatformSpecificWindowBuilderAttributes,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(pub(crate) u32);

impl WindowId {
  pub fn dummy() -> Self {
    WindowId(u32::MAX)
  }
}

/// An icon used for the window titlebar, taskbar, etc.
#[derive(Debug, Clone)]
pub struct PlatformIcon {
  raw: Vec<u8>,
  width: i32,
  height: i32,
  row_stride: i32,
}

impl From<PlatformIcon> for Pixbuf {
  fn from(icon: PlatformIcon) -> Self {
    Pixbuf::from_mut_slice(
      icon.raw,
      gdk_pixbuf::Colorspace::Rgb,
      true,
      8,
      icon.width,
      icon.height,
      icon.row_stride,
    )
  }
}

impl PlatformIcon {
  /// Creates an `Icon` from 32bpp RGBA data.
  ///
  /// The length of `rgba` must be divisible by 4, and `width * height` must equal
  /// `rgba.len() / 4`. Otherwise, this will return a `BadIcon` error.
  pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, BadIcon> {
    let row_stride =
      Pixbuf::calculate_rowstride(Colorspace::Rgb, true, 8, width as i32, height as i32);
    Ok(Self {
      raw: rgba,
      width: width as i32,
      height: height as i32,
      row_stride,
    })
  }
}

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
}

impl Window {
  pub(crate) fn new<T>(
    event_loop_window_target: &EventLoopWindowTarget<T>,
    attributes: WindowAttributes,
    pl_attribs: PlatformSpecificWindowBuilderAttributes,
  ) -> Result<Self, RootOsError> {
    let app = &event_loop_window_target.app;
    let window_requests_tx = event_loop_window_target.window_requests_tx.clone();
    let window = gtk::ApplicationWindow::new(app);
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
    window.set_resizable(attributes.resizable);
    if attributes.resizable {
      window.set_default_size(width, height);
    } else {
      window.set_size_request(width, height);
    }

    // Set Min/Max Size
    let geom_mask = (if attributes.min_inner_size.is_some() {
      gdk::WindowHints::MIN_SIZE
    } else {
      gdk::WindowHints::empty()
    }) | (if attributes.max_inner_size.is_some() {
      gdk::WindowHints::MAX_SIZE
    } else {
      gdk::WindowHints::empty()
    });
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

    // Set Transparent
    if attributes.transparent {
      if let Some(screen) = window.screen() {
        if let Some(visual) = screen.rgba_visual() {
          window.set_visual(Some(&visual));
        }
      }

      window.connect_draw(|_, cr| {
        cr.set_source_rgba(0., 0., 0., 0.);
        cr.set_operator(cairo::Operator::Source);
        let _ = cr.paint();
        cr.set_operator(cairo::Operator::Over);
        Inhibit(false)
      });
      window.set_app_paintable(true);
    }

    // We always create a box and allocate menubar, so if they set_menu after creation
    // we can inject the menubar without re-redendering the whole window
    let window_box = gtk::Box::new(Orientation::Vertical, 0);
    window.add(&window_box);

    let mut menu_bar = gtk::MenuBar::new();
    window_box.pack_start(&menu_bar, false, false, 0);
    if let Some(window_menu) = attributes.window_menu {
      window_menu.generate_menu(&mut menu_bar, &window_requests_tx, &accel_group, window_id);
    }

    // Rest attributes
    window.set_title(&attributes.title);
    if let Some(Fullscreen::Borderless(Some(f))) = &attributes.fullscreen {
      let number = f.inner.number;
      let screen = window.display().default_screen();
      window.fullscreen_on_monitor(&screen, number);
    }
    if attributes.maximized {
      window.maximize();
    }
    window.set_visible(attributes.visible);
    window.set_decorated(attributes.decorations);

    window.set_keep_above(attributes.always_on_top);
    if let Some(icon) = attributes.window_icon {
      window.set_icon(Some(&icon.inner.into()));
    }

    if attributes.visible {
      window.show_all();
    } else {
      window.hide();
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

    if let Err(e) = window_requests_tx.send((window_id, WindowRequest::WireUpEvents)) {
      log::warn!("Fail to send wire up events request: {}", e);
    }

    if let Err(e) = window_requests_tx.send((window_id, WindowRequest::Redraw)) {
      log::warn!("Fail to send redraw request: {}", e);
    }

    let win = Self {
      window_id,
      window,
      window_requests_tx,
      accel_group,
      menu_bar,
      scale_factor,
      position,
      size,
      maximized,
      minimized,
      fullscreen: RefCell::new(attributes.fullscreen),
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
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::Redraw))
    {
      log::warn!("Fail to send redraw request: {}", e);
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
    if let Some(size) = min_size {
      let (min_width, min_height) = size.into().to_logical::<i32>(self.scale_factor()).into();

      if let Err(e) = self.window_requests_tx.send((
        self.window_id,
        WindowRequest::MinSize((min_width, min_height)),
      )) {
        log::warn!("Fail to send min size request: {}", e);
      }
    }
  }
  pub fn set_max_inner_size<S: Into<Size>>(&self, max_size: Option<S>) {
    if let Some(size) = max_size {
      let (max_width, max_height) = size.into().to_logical::<i32>(self.scale_factor()).into();

      if let Err(e) = self.window_requests_tx.send((
        self.window_id,
        WindowRequest::MaxSize((max_width, max_height)),
      )) {
        log::warn!("Fail to send max size request: {}", e);
      }
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

  pub fn set_resizable(&self, resizable: bool) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::Resizable(resizable)))
    {
      log::warn!("Fail to send resizable request: {}", e);
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

  pub fn is_resizable(&self) -> bool {
    self.window.is_resizable()
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

  pub fn set_cursor_icon(&self, cursor: CursorIcon) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::CursorIcon(Some(cursor))))
    {
      log::warn!("Fail to send cursor icon request: {}", e);
    }
  }

  pub fn set_cursor_position<P: Into<Position>>(&self, _position: P) -> Result<(), ExternalError> {
    todo!()
  }

  pub fn set_cursor_grab(&self, _grab: bool) -> Result<(), ExternalError> {
    todo!()
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

  pub fn current_monitor(&self) -> Option<RootMonitorHandle> {
    let screen = self.window.display().default_screen();
    let window = self.window.window().unwrap();
    #[allow(deprecated)] // Gtk3 Window only accepts Gdkscreen
    let number = screen.monitor_at_window(&window);
    let handle = MonitorHandle::new(&self.window.display(), number);
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
    let screen = self.window.display().default_screen();
    #[allow(deprecated)] // Gtk3 Window only accepts Gdkscreen
    let number = screen.primary_monitor();
    let handle = MonitorHandle::new(&self.window.display(), number);
    Some(RootMonitorHandle { inner: handle })
  }

  pub fn raw_window_handle(&self) -> RawWindowHandle {
    // TODO: add wayland support
    let mut handle = XlibHandle::empty();
    unsafe {
      if let Some(window) = self.window.window() {
        handle.window = gdk_x11_sys::gdk_x11_window_get_xid(window.as_ptr() as *mut _);
      }
      if let Ok(xlib) = x11_dl::xlib::Xlib::open() {
        handle.display = (xlib.XOpenDisplay)(std::ptr::null()) as _;
      }
    }
    RawWindowHandle::Xlib(handle)
  }

  pub(crate) fn set_skip_taskbar(&self, skip: bool) {
    if let Err(e) = self
      .window_requests_tx
      .send((self.window_id, WindowRequest::SetSkipTaskbar(skip)))
    {
      log::warn!("Fail to send skip taskbar request: {}", e);
    }
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
  MinSize((i32, i32)),
  MaxSize((i32, i32)),
  Visible(bool),
  Focus,
  Resizable(bool),
  Minimized(bool),
  Maximized(bool),
  DragWindow,
  Fullscreen(Option<Fullscreen>),
  Decorations(bool),
  AlwaysOnTop(bool),
  WindowIcon(Option<Icon>),
  UserAttention(Option<UserAttentionType>),
  SetSkipTaskbar(bool),
  CursorIcon(Option<CursorIcon>),
  WireUpEvents,
  Redraw,
  Menu((Option<MenuItem>, Option<MenuId>)),
  SetMenu((Option<menu::Menu>, AccelGroup, gtk::MenuBar)),
  GlobalHotKey(u16),
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

  #[rustfmt::skip]
  let result = (LEFT * (if cx < (left + BORDERLESS_RESIZE_INSET) { 1 } else { 0 }))
    | (RIGHT * (if cx >= (right - BORDERLESS_RESIZE_INSET) { 1 } else { 0 }))
    | (TOP * (if cy < (top + BORDERLESS_RESIZE_INSET) { 1 } else { 0 }))
    | (BOTTOM * (if cy >= (bottom - BORDERLESS_RESIZE_INSET) { 1 } else { 0 }));

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
