// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  cell::RefCell,
  collections::{HashSet, VecDeque},
  error::Error,
  process,
  rc::Rc,
  sync::{mpsc::SendError, Mutex},
  time::Instant,
};

use gdk::{Cursor, CursorType, WindowExt, WindowState};
use gio::{prelude::*, Cancellable};
use glib::{source::Priority, Continue, MainContext};
use gtk::{prelude::*, AboutDialog, ApplicationWindow, Inhibit};

#[cfg(any(feature = "menu", feature = "tray"))]
use glib::Cast;
#[cfg(any(feature = "menu", feature = "tray"))]
use gtk::{Clipboard, Entry};

use crate::{
  dpi::{PhysicalPosition, PhysicalSize},
  event::{
    DeviceId as RootDeviceId, ElementState, Event, ModifiersState, MouseButton, StartCause,
    WindowEvent,
  },
  event_loop::{ControlFlow, EventLoopClosed, EventLoopWindowTarget as RootELW},
  menu::{MenuItem, MenuType},
  monitor::MonitorHandle as RootMonitorHandle,
  window::{CursorIcon, WindowId as RootWindowId},
};

use super::{
  menu,
  monitor::MonitorHandle,
  window::{WindowId, WindowRequest},
  DeviceId,
};

pub struct EventLoopWindowTarget<T> {
  /// Gtk application
  pub(crate) app: gtk::Application,
  /// Window Ids of the application
  pub(crate) windows: Rc<RefCell<HashSet<WindowId>>>,
  /// Window requests sender
  pub(crate) window_requests_tx: glib::Sender<(WindowId, WindowRequest)>,
  _marker: std::marker::PhantomData<T>,
}

impl<T> EventLoopWindowTarget<T> {
  #[inline]
  pub fn available_monitors(&self) -> VecDeque<MonitorHandle> {
    todo!()
  }

  #[inline]
  pub fn primary_monitor(&self) -> Option<RootMonitorHandle> {
    todo!()
  }
}

pub struct EventLoop<T: 'static> {
  /// Window target.
  window_target: RootELW<T>,
  /// User event sender for EventLoopProxy
  user_event_tx: glib::Sender<T>,
  /// User event receiver
  user_event_rx: glib::Receiver<T>,
  /// Window requests receiver
  window_requests_rx: glib::Receiver<(WindowId, WindowRequest)>,
}

impl<T: 'static> EventLoop<T> {
  pub fn new() -> EventLoop<T> {
    assert_is_main_thread("new_any_thread");
    EventLoop::new_gtk().expect("Failed to initialize any backend!")
  }

  fn new_gtk() -> Result<EventLoop<T>, Box<dyn Error>> {
    let app = gtk::Application::new(None, gio::ApplicationFlags::empty())?;
    let cancellable: Option<&Cancellable> = None;
    app.register(cancellable)?;

    // Create event loop window target.
    let (window_requests_tx, window_requests_rx) = glib::MainContext::channel(Priority::default());
    let window_target = EventLoopWindowTarget {
      app,
      windows: Rc::new(RefCell::new(HashSet::new())),
      window_requests_tx,
      _marker: std::marker::PhantomData,
    };

    // Create user event channel
    let (user_event_tx, user_event_rx) = glib::MainContext::channel(Priority::default());

    // Create event loop itself.
    let event_loop = Self {
      window_target: RootELW {
        p: window_target,
        _marker: std::marker::PhantomData,
      },
      user_event_tx,
      user_event_rx,
      window_requests_rx,
    };

    Ok(event_loop)
  }

  #[inline]
  pub fn run<F>(self, callback: F) -> !
  where
    F: FnMut(Event<'_, T>, &RootELW<T>, &mut ControlFlow) + 'static,
  {
    self.run_return(callback);
    process::exit(0)
  }

  pub(crate) fn run_return<F>(self, mut callback: F)
  where
    F: FnMut(Event<'_, T>, &RootELW<T>, &mut ControlFlow) + 'static,
  {
    let mut control_flow = ControlFlow::default();
    let window_target = self.window_target;
    let (event_tx, event_rx) = glib::MainContext::channel::<Event<'_, T>>(Priority::default());

    // Send StartCause::Init event
    let event_tx_ = event_tx.clone();
    window_target.p.app.connect_activate(move |_| {
      if let Err(e) = event_tx_.send(Event::NewEvents(StartCause::Init)) {
        log::warn!("Failed to send init event to event channel: {}", e);
      }
    });
    window_target.p.app.activate();

    let context = MainContext::default();
    context.push_thread_default();

    // User event
    let event_tx_ = event_tx.clone();
    self.user_event_rx.attach(Some(&context), move |event| {
      if let Err(e) = event_tx_.send(Event::UserEvent(event)) {
        log::warn!("Failed to send user event to event channel: {}", e);
      }
      Continue(true)
    });

    // Window Request
    let app = window_target.p.app.clone();
    let windows = window_target.p.windows.clone();
    let window_requests_tx = window_target.p.window_requests_tx.clone();
    self
      .window_requests_rx
      .attach(Some(&context), move |(id, request)| {
        if let Some(window) = app.get_window_by_id(id.0) {
          match request {
            WindowRequest::Title(title) => window.set_title(&title),
            WindowRequest::Position((x, y)) => window.move_(x, y),
            WindowRequest::Size((w, h)) => window.resize(w, h),
            WindowRequest::MinSize((min_width, min_height)) => window
              .set_geometry_hints::<ApplicationWindow>(
                None,
                Some(&gdk::Geometry {
                  min_width,
                  min_height,
                  max_width: 0,
                  max_height: 0,
                  base_width: 0,
                  base_height: 0,
                  width_inc: 0,
                  height_inc: 0,
                  min_aspect: 0f64,
                  max_aspect: 0f64,
                  win_gravity: gdk::Gravity::Center,
                }),
                gdk::WindowHints::MIN_SIZE,
              ),
            WindowRequest::MaxSize((max_width, max_height)) => window
              .set_geometry_hints::<ApplicationWindow>(
                None,
                Some(&gdk::Geometry {
                  min_width: 0,
                  min_height: 0,
                  max_width,
                  max_height,
                  base_width: 0,
                  base_height: 0,
                  width_inc: 0,
                  height_inc: 0,
                  min_aspect: 0f64,
                  max_aspect: 0f64,
                  win_gravity: gdk::Gravity::Center,
                }),
                gdk::WindowHints::MAX_SIZE,
              ),
            WindowRequest::Visible(visible) => {
              if visible {
                window.show_all();
              } else {
                window.hide();
              }
            }
            WindowRequest::Focus => {
              // FIXME: replace with present_with_timestamp
              window.present();
            }
            WindowRequest::Resizable(resizable) => window.set_resizable(resizable),
            WindowRequest::Minimized(minimized) => {
              if minimized {
                window.iconify();
              } else {
                window.deiconify();
              }
            }
            WindowRequest::Maximized(maximized) => {
              if maximized {
                window.maximize();
              } else {
                window.unmaximize();
              }
            }
            WindowRequest::DragWindow => {
              let display = window.get_display();
              if let Some(cursor) = display
                .get_device_manager()
                .and_then(|device_manager| device_manager.get_client_pointer())
              {
                let (_, x, y) = cursor.get_position();
                window.begin_move_drag(1, x, y, 0);
              }
            }
            WindowRequest::Fullscreen(fullscreen) => match fullscreen {
              Some(_) => window.fullscreen(),
              None => window.unfullscreen(),
            },
            WindowRequest::Decorations(decorations) => window.set_decorated(decorations),
            WindowRequest::AlwaysOnTop(always_on_top) => window.set_keep_above(always_on_top),
            WindowRequest::WindowIcon(window_icon) => {
              if let Some(icon) = window_icon {
                window.set_icon(Some(&icon.inner.into()));
              }
            }
            WindowRequest::UserAttention(request_type) => {
              if request_type.is_some() {
                window.set_urgency_hint(true)
              }
            }
            WindowRequest::SetSkipTaskbar(skip) => window.set_skip_taskbar_hint(skip),
            WindowRequest::CursorIcon(cursor) => {
              if let Some(gdk_window) = window.get_window() {
                let display = window.get_display();
                match cursor {
                  Some(cr) => gdk_window.set_cursor(
                    Cursor::from_name(
                      &display,
                      match cr {
                        CursorIcon::Crosshair => "crosshair",
                        CursorIcon::Hand => "pointer",
                        CursorIcon::Arrow => "crosshair",
                        CursorIcon::Move => "move",
                        CursorIcon::Text => "text",
                        CursorIcon::Wait => "wait",
                        CursorIcon::Help => "help",
                        CursorIcon::Progress => "progress",
                        CursorIcon::NotAllowed => "not-allowed",
                        CursorIcon::ContextMenu => "context-menu",
                        CursorIcon::Cell => "cell",
                        CursorIcon::VerticalText => "vertical-text",
                        CursorIcon::Alias => "alias",
                        CursorIcon::Copy => "copy",
                        CursorIcon::NoDrop => "no-drop",
                        CursorIcon::Grab => "grab",
                        CursorIcon::Grabbing => "grabbing",
                        CursorIcon::AllScroll => "all-scroll",
                        CursorIcon::ZoomIn => "zoom-in",
                        CursorIcon::ZoomOut => "zoom-out",
                        CursorIcon::EResize => "e-resize",
                        CursorIcon::NResize => "n-resize",
                        CursorIcon::NeResize => "ne-resize",
                        CursorIcon::NwResize => "nw-resize",
                        CursorIcon::SResize => "s-resize",
                        CursorIcon::SeResize => "se-resize",
                        CursorIcon::SwResize => "sw-resize",
                        CursorIcon::WResize => "w-resize",
                        CursorIcon::EwResize => "ew-resize",
                        CursorIcon::NsResize => "ns-resize",
                        CursorIcon::NeswResize => "nesw-resize",
                        CursorIcon::NwseResize => "nwse-resize",
                        CursorIcon::ColResize => "col-resize",
                        CursorIcon::RowResize => "row-resize",
                        CursorIcon::Default => "default",
                      },
                    )
                    .as_ref(),
                  ),
                  None => gdk_window.set_cursor(Some(&Cursor::new_for_display(
                    &display,
                    CursorType::BlankCursor,
                  ))),
                }
              };
            }
            WindowRequest::WireUpEvents => {
              let windows_rc = windows.clone();
              let tx_clone = event_tx.clone();

              window.connect_delete_event(move |_, _| {
                windows_rc.borrow_mut().remove(&id);
                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::CloseRequested,
                }) {
                  log::warn!("Failed to send window close event to event channel: {}", e);
                }
                Inhibit(false)
              });

              let tx_clone = event_tx.clone();
              window.connect_configure_event(move |_, event| {
                let (x, y) = event.get_position();
                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::Moved(PhysicalPosition::new(x, y)),
                }) {
                  log::warn!("Failed to send window moved event to event channel: {}", e);
                }

                let (w, h) = event.get_size();
                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::Resized(PhysicalSize::new(w, h)),
                }) {
                  log::warn!(
                    "Failed to send window resized event to event channel: {}",
                    e
                  );
                }
                false
              });

              let tx_clone = event_tx.clone();
              window.connect_window_state_event(move |_window, event| {
                let state = event.get_new_window_state();

                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::Focused(state.contains(WindowState::FOCUSED)),
                }) {
                  log::warn!(
                    "Failed to send window focused event to event channel: {}",
                    e
                  );
                }
                Inhibit(false)
              });

              let tx_clone = event_tx.clone();
              window.connect_destroy_event(move |_, _| {
                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::Destroyed,
                }) {
                  log::warn!(
                    "Failed to send window destroyed event to event channel: {}",
                    e
                  );
                }
                Inhibit(false)
              });

              let tx_clone = event_tx.clone();
              window.connect_enter_notify_event(move |_, _| {
                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::CursorEntered {
                    // FIXME: currently we use a dummy device id, find if we can get device id from gtk
                    device_id: RootDeviceId(DeviceId(0)),
                  },
                }) {
                  log::warn!(
                    "Failed to send cursor entered event to event channel: {}",
                    e
                  );
                }
                Inhibit(false)
              });

              let tx_clone = event_tx.clone();
              window.connect_motion_notify_event(move |window, _| {
                let display = window.get_display();
                if let Some(cursor) = display
                  .get_device_manager()
                  .and_then(|device_manager| device_manager.get_client_pointer())
                {
                  let (_, x, y) = cursor.get_position();
                  if let Err(e) = tx_clone.send(Event::WindowEvent {
                    window_id: RootWindowId(id),
                    event: WindowEvent::CursorMoved {
                      position: PhysicalPosition::new(x as f64, y as f64),
                      // FIXME: currently we use a dummy device id, find if we can get device id from gtk
                      device_id: RootDeviceId(DeviceId(0)),
                      // this field is depracted so it is fine to pass empty state
                      modifiers: ModifiersState::empty(),
                    },
                  }) {
                    log::warn!("Failed to send cursor moved event to event channel: {}", e);
                  }
                }
                Inhibit(false)
              });

              let tx_clone = event_tx.clone();
              window.connect_leave_notify_event(move |_, _| {
                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::CursorLeft {
                    // FIXME: currently we use a dummy device id, find if we can get device id from gtk
                    device_id: RootDeviceId(DeviceId(0)),
                  },
                }) {
                  log::warn!("Failed to send cursor left event to event channel: {}", e);
                }
                Inhibit(false)
              });

              let tx_clone = event_tx.clone();
              window.connect_button_press_event(move |_, event| {
                let button = event.get_button();
                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::MouseInput {
                    button: match button {
                      1 => MouseButton::Left,
                      2 => MouseButton::Middle,
                      3 => MouseButton::Right,
                      _ => MouseButton::Other(button as u16),
                    },
                    state: ElementState::Pressed,
                    // FIXME: currently we use a dummy device id, find if we can get device id from gtk
                    device_id: RootDeviceId(DeviceId(0)),
                    // this field is depracted so it is fine to pass empty state
                    modifiers: ModifiersState::empty(),
                  },
                }) {
                  log::warn!(
                    "Failed to send mouse input preseed event to event channel: {}",
                    e
                  );
                }
                Inhibit(false)
              });

              let tx_clone = event_tx.clone();
              window.connect_button_release_event(move |_, event| {
                let button = event.get_button();
                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::MouseInput {
                    button: match button {
                      1 => MouseButton::Left,
                      2 => MouseButton::Middle,
                      3 => MouseButton::Right,
                      _ => MouseButton::Other(button as u16),
                    },
                    state: ElementState::Released,
                    // FIXME: currently we use a dummy device id, find if we can get device id from gtk
                    device_id: RootDeviceId(DeviceId(0)),
                    // this field is depracted so it is fine to pass empty state
                    modifiers: ModifiersState::empty(),
                  },
                }) {
                  log::warn!(
                    "Failed to send mouse input released event to event channel: {}",
                    e
                  );
                }
                Inhibit(false)
              });
            }
            WindowRequest::Redraw => window.queue_draw(),
            WindowRequest::Menu(m) => {
              match m {
                MenuItem::Custom(c) => {
                  if let Err(e) = event_tx.send(Event::MenuEvent {
                    menu_id: c.id,
                    origin: MenuType::Menubar,
                  }) {
                    log::warn!("Failed to send menu event to event channel: {}", e);
                  }
                }
                MenuItem::About(_) => {
                  let about = AboutDialog::new();
                  about.show_all();
                  app.add_window(&about);
                }
                MenuItem::Hide => window.hide(),
                MenuItem::CloseWindow => window.close(),
                MenuItem::Quit => {
                  if let Err(e) = event_tx.send(Event::LoopDestroyed) {
                    log::warn!(
                      "Failed to send loop destroyed event to event channel: {}",
                      e
                    );
                  }
                }
                #[cfg(any(feature = "menu", feature = "tray"))]
                MenuItem::Cut => {
                  if let Some(widget) = window.get_focus() {
                    if widget.has_focus() {
                      if let Some(view) = widget.dynamic_cast_ref::<sourceview::View>() {
                        if let Some(clipboard) = Clipboard::get_default(&widget.get_display()) {
                          if let Some(buf) = view.get_buffer() {
                            buf.cut_clipboard(&clipboard, true);
                          }
                        }
                      } else if let Some(entry) = widget.dynamic_cast_ref::<Entry>() {
                        entry.cut_clipboard();
                      }
                    }
                  }
                }
                #[cfg(any(feature = "menu", feature = "tray"))]
                MenuItem::Copy => {
                  if let Some(widget) = window.get_focus() {
                    if widget.has_focus() {
                      if let Some(view) = widget.dynamic_cast_ref::<sourceview::View>() {
                        if let Some(clipboard) = Clipboard::get_default(&widget.get_display()) {
                          if let Some(buf) = view.get_buffer() {
                            buf.copy_clipboard(&clipboard);
                          }
                        }
                      } else if let Some(entry) = widget.dynamic_cast_ref::<Entry>() {
                        entry.copy_clipboard();
                      }
                    }
                  }
                }
                #[cfg(any(feature = "menu", feature = "tray"))]
                MenuItem::Paste => {
                  if let Some(widget) = window.get_focus() {
                    if widget.has_focus() {
                      if let Some(view) = widget.dynamic_cast_ref::<sourceview::View>() {
                        if let Some(clipboard) = Clipboard::get_default(&widget.get_display()) {
                          if let Some(buf) = view.get_buffer() {
                            buf.paste_clipboard(&clipboard, None, true);
                          }
                        }
                      } else if let Some(entry) = widget.dynamic_cast_ref::<Entry>() {
                        entry.paste_clipboard();
                      }
                    }
                  }
                }
                #[cfg(any(feature = "menu", feature = "tray"))]
                MenuItem::SelectAll => {
                  if let Some(widget) = window.get_focus() {
                    if widget.has_focus() {
                      if let Some(view) = widget.dynamic_cast_ref::<sourceview::View>() {
                        if let Some(buf) = view.get_buffer() {
                          buf.select_range(&buf.get_start_iter(), &buf.get_end_iter());
                        }
                      } else if let Some(entry) = widget.dynamic_cast_ref::<Entry>() {
                        entry.select_region(0, -1);
                      }
                    }
                  }
                }
                // TODO toggle fullscreen
                MenuItem::EnterFullScreen => window.fullscreen(),
                MenuItem::Minimize => window.iconify(),
                _ => {}
              }
            }
            WindowRequest::SetMenu((menus, accel_group, menu)) => {
              for i in menu.get_children() {
                menu.remove(&i);
              }

              if let Some(menus) = menus {
                let menubar = menu::initialize(id, menus, &window_requests_tx, &accel_group);
                menu.pack_start(&menubar, false, false, 0);
                menu.show_all();
              }
            }
          }
        } else if id == WindowId::dummy() {
          if let WindowRequest::Menu(MenuItem::Custom(c)) = request {
            if let Err(e) = event_tx.send(Event::MenuEvent {
              menu_id: c.id,
              origin: MenuType::SystemTray,
            }) {
              log::warn!("Failed to send status bar event to event channel: {}", e);
            }
          }
        }
        Continue(true)
      });

    // Event control flow
    let events = Rc::new(Mutex::new(Vec::new()));
    let events_ = events.clone();
    event_rx.attach(Some(&context), move |event| {
      let mut e = events_.lock().unwrap();
      e.push(event);
      Continue(true)
    });

    loop {
      match control_flow {
        ControlFlow::Exit => {
          callback(Event::LoopDestroyed, &window_target, &mut control_flow);
          break;
        }
        ControlFlow::Wait => {
          let mut e = events.lock().unwrap();
          if !e.is_empty() {
            callback(
              Event::NewEvents(StartCause::WaitCancelled {
                start: Instant::now(),
                requested_resume: None,
              }),
              &window_target,
              &mut control_flow,
            );

            for event in e.drain(..) {
              match event {
                Event::LoopDestroyed => control_flow = ControlFlow::Exit,
                _ => callback(event, &window_target, &mut control_flow),
              }
            }

            if control_flow != ControlFlow::Exit {
              callback(Event::MainEventsCleared, &window_target, &mut control_flow);
            }
          }
        }
        ControlFlow::WaitUntil(requested_resume) => {
          let mut e = events.lock().unwrap();
          let start = Instant::now();
          if start >= requested_resume {
            callback(
              Event::NewEvents(StartCause::ResumeTimeReached {
                start,
                requested_resume,
              }),
              &window_target,
              &mut control_flow,
            );

            for event in e.drain(..) {
              match event {
                Event::LoopDestroyed => control_flow = ControlFlow::Exit,
                _ => callback(event, &window_target, &mut control_flow),
              }
            }

            if control_flow != ControlFlow::Exit {
              callback(Event::MainEventsCleared, &window_target, &mut control_flow);
            }
          } else if !e.is_empty() {
            callback(
              Event::NewEvents(StartCause::WaitCancelled {
                start,
                requested_resume: Some(requested_resume),
              }),
              &window_target,
              &mut control_flow,
            );

            for event in e.drain(..) {
              match event {
                Event::LoopDestroyed => control_flow = ControlFlow::Exit,
                _ => callback(event, &window_target, &mut control_flow),
              }
            }

            if control_flow != ControlFlow::Exit {
              callback(Event::MainEventsCleared, &window_target, &mut control_flow);
            }
          }
        }
        ControlFlow::Poll => {
          let mut e = events.lock().unwrap();
          callback(
            Event::NewEvents(StartCause::Poll),
            &window_target,
            &mut control_flow,
          );
          for event in e.drain(..) {
            match event {
              Event::LoopDestroyed => control_flow = ControlFlow::Exit,
              _ => callback(event, &window_target, &mut control_flow),
            }
          }
          callback(Event::MainEventsCleared, &window_target, &mut control_flow);
        }
      }

      gtk::main_iteration();
    }
    context.pop_thread_default();
  }

  #[inline]
  pub fn window_target(&self) -> &RootELW<T> {
    &self.window_target
  }

  /// Creates an `EventLoopProxy` that can be used to dispatch user events to the main event loop.
  pub fn create_proxy(&self) -> EventLoopProxy<T> {
    EventLoopProxy {
      user_event_tx: self.user_event_tx.clone(),
    }
  }
}

/// Used to send custom events to `EventLoop`.
#[derive(Debug)]
pub struct EventLoopProxy<T: 'static> {
  user_event_tx: glib::Sender<T>,
}

impl<T: 'static> Clone for EventLoopProxy<T> {
  fn clone(&self) -> Self {
    Self {
      user_event_tx: self.user_event_tx.clone(),
    }
  }
}

impl<T: 'static> EventLoopProxy<T> {
  /// Send an event to the `EventLoop` from which this proxy was created. This emits a
  /// `UserEvent(event)` event in the event loop, where `event` is the value passed to this
  /// function.
  ///
  /// Returns an `Err` if the associated `EventLoop` no longer exists.
  pub fn send_event(&self, event: T) -> Result<(), EventLoopClosed<T>> {
    self
      .user_event_tx
      .send(event)
      .map_err(|SendError(error)| EventLoopClosed(error))
  }
}

fn assert_is_main_thread(suggested_method: &str) {
  if !is_main_thread() {
    panic!(
      "Initializing the event loop outside of the main thread is a significant \
             cross-platform compatibility hazard. If you really, absolutely need to create an \
             EventLoop on a different thread, please use the `EventLoopExtUnix::{}` function.",
      suggested_method
    );
  }
}

#[cfg(target_os = "linux")]
fn is_main_thread() -> bool {
  use libc::{c_long, getpid, syscall, SYS_gettid};

  unsafe { syscall(SYS_gettid) == getpid() as c_long }
}

#[cfg(any(target_os = "dragonfly", target_os = "freebsd", target_os = "openbsd"))]
fn is_main_thread() -> bool {
  use libc::pthread_main_np;

  unsafe { pthread_main_np() == 1 }
}

#[cfg(target_os = "netbsd")]
fn is_main_thread() -> bool {
  std::thread::current().name() == Some("main")
}
