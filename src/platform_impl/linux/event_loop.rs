// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  cell::RefCell,
  collections::{HashSet, VecDeque},
  error::Error,
  process,
  rc::Rc,
  sync::mpsc::SendError,
  time::Instant,
};

use gdk::{Cursor, CursorType, EventKey, EventMask, WindowEdge, WindowState};
use gio::{prelude::*, Cancellable};
use glib::{source::Priority, Continue, MainContext};
use gtk::{prelude::*, AboutDialog, Inhibit};

use crate::{
  accelerator::AcceleratorId,
  dpi::{LogicalPosition, LogicalSize},
  event::{ElementState, Event, MouseButton, StartCause, WindowEvent},
  event_loop::{ControlFlow, EventLoopClosed, EventLoopWindowTarget as RootELW},
  keyboard::ModifiersState,
  menu::{MenuItem, MenuType},
  monitor::MonitorHandle as RootMonitorHandle,
  platform_impl::platform::{window::hit_test, DEVICE_ID},
  window::{CursorIcon, Fullscreen, WindowId as RootWindowId},
};

use super::{
  keyboard,
  monitor::MonitorHandle,
  window::{WindowId, WindowRequest},
};

#[derive(Clone)]
pub struct EventLoopWindowTarget<T> {
  /// Gdk display
  pub(crate) display: gdk::Display,
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
    let mut handles = VecDeque::new();
    let display = &self.display;
    let numbers = display.n_monitors();

    for i in 0..numbers {
      let monitor = MonitorHandle::new(display, i);
      handles.push_back(monitor);
    }

    handles
  }

  #[inline]
  pub fn primary_monitor(&self) -> Option<RootMonitorHandle> {
    let screen = self.display.default_screen();
    #[allow(deprecated)] // Gtk3 Window only accepts Gdkscreen
    let number = screen.primary_monitor();
    let handle = MonitorHandle::new(&self.display, number);
    Some(RootMonitorHandle { inner: handle })
  }
}

pub struct EventLoop<T: 'static> {
  /// Window target.
  window_target: RootELW<T>,
  /// User event sender for EventLoopProxy
  user_event_tx: glib::Sender<T>,
  /// Event queue of EventLoop
  events: crossbeam_channel::Receiver<Event<'static, T>>,
  /// Draw queue of EventLoop
  draws: crossbeam_channel::Receiver<WindowId>,
}

impl<T: 'static> EventLoop<T> {
  pub fn new() -> EventLoop<T> {
    assert_is_main_thread("new_any_thread");
    EventLoop::new_any_thread()
  }

  pub fn new_any_thread() -> EventLoop<T> {
    let context = MainContext::default();
    context
      .with_thread_default(|| EventLoop::new_gtk().expect("Failed to initialize gtk backend!"))
      .expect("Failed to initialize gtk backend!")
  }

  fn new_gtk() -> Result<EventLoop<T>, Box<dyn Error>> {
    let context = MainContext::default();
    let app = gtk::Application::new(None, gio::ApplicationFlags::empty());
    let app_ = app.clone();
    let cancellable: Option<&Cancellable> = None;
    app.register(cancellable)?;

    // Send StartCause::Init event
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    let (draw_tx, draw_rx) = crossbeam_channel::unbounded();
    let event_tx_ = event_tx.clone();
    app.connect_activate(move |_| {
      if let Err(e) = event_tx_.send(Event::NewEvents(StartCause::Init)) {
        log::warn!("Failed to send init event to event channel: {}", e);
      }
    });

    // Create event loop window target.
    let (window_requests_tx, window_requests_rx) = glib::MainContext::channel(Priority::default());
    let window_requests_tx_ = window_requests_tx.clone();
    let display = gdk::Display::default()
      .expect("GdkDisplay not found. This usually means `gkt_init` hasn't called yet.");
    let window_target = EventLoopWindowTarget {
      display,
      app,
      windows: Rc::new(RefCell::new(HashSet::new())),
      window_requests_tx,
      _marker: std::marker::PhantomData,
    };

    // Create user event channel
    let (user_event_tx, user_event_rx) = glib::MainContext::channel(Priority::default());
    let event_tx_ = event_tx.clone();
    user_event_rx.attach(Some(&context), move |event| {
      if let Err(e) = event_tx_.send(Event::UserEvent(event)) {
        log::warn!("Failed to send user event to event channel: {}", e);
      }
      Continue(true)
    });

    // Window Request
    window_requests_rx.attach(Some(&context), move |(id, request)| {
      if let Some(window) = app_.window_by_id(id.0) {
        match request {
          WindowRequest::Title(title) => window.set_title(&title),
          WindowRequest::Position((x, y)) => window.move_(x, y),
          WindowRequest::Size((w, h)) => window.resize(w, h),
          WindowRequest::MinSize((min_width, min_height)) => {
            let picky_none: Option<&gtk::Window> = None;
            window.set_geometry_hints(
              picky_none,
              Some(&gdk::Geometry::new(
                min_width,
                min_height,
                0,
                0,
                0,
                0,
                0,
                0,
                0f64,
                0f64,
                gdk::Gravity::Center,
              )),
              gdk::WindowHints::MIN_SIZE,
            )
          }
          WindowRequest::MaxSize((max_width, max_height)) => {
            let picky_none: Option<&gtk::Window> = None;
            window.set_geometry_hints(
              picky_none,
              Some(&gdk::Geometry::new(
                0,
                0,
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
              gdk::WindowHints::MAX_SIZE,
            )
          }
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
            let display = window.display();
            if let Some(cursor) = display
              .default_seat()
              .and_then(|device_manager| device_manager.pointer())
            {
              let (_, x, y) = cursor.position();
              window.begin_move_drag(1, x, y, 0);
            }
          }
          WindowRequest::Fullscreen(fullscreen) => match fullscreen {
            Some(f) => {
              if let Fullscreen::Borderless(m) = f {
                if let Some(monitor) = m {
                  let number = monitor.inner.number;
                  let screen = window.display().default_screen();
                  window.fullscreen_on_monitor(&screen, number);
                } else {
                  window.fullscreen();
                }
              }
            }
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
            window.set_urgency_hint(request_type.is_some())
          }
          WindowRequest::SetSkipTaskbar(skip) => {
            window.set_skip_taskbar_hint(skip);
            window.set_skip_pager_hint(skip)
          }
          WindowRequest::CursorIcon(cursor) => {
            if let Some(gdk_window) = window.window() {
              let display = window.display();
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
                None => gdk_window
                  .set_cursor(Cursor::for_display(&display, CursorType::BlankCursor).as_ref()),
              }
            };
          }
          WindowRequest::WireUpEvents => {
            window.add_events(
              EventMask::POINTER_MOTION_MASK
                | EventMask::BUTTON1_MOTION_MASK
                | EventMask::BUTTON_PRESS_MASK
                | EventMask::TOUCH_MASK
                | EventMask::STRUCTURE_MASK
                | EventMask::FOCUS_CHANGE_MASK,
            );

            // Resizing `decorations: false` aka borderless
            window.connect_motion_notify_event(|window, event| {
              if !window.is_decorated() && window.is_resizable() {
                if let Some(window) = window.window() {
                  let (cx, cy) = event.root();
                  let edge = hit_test(&window, cx, cy);
                  window.set_cursor(
                    Cursor::from_name(
                      &window.display(),
                      match edge {
                        WindowEdge::North => "n-resize",
                        WindowEdge::South => "s-resize",
                        WindowEdge::East => "e-resize",
                        WindowEdge::West => "w-resize",
                        WindowEdge::NorthWest => "nw-resize",
                        WindowEdge::NorthEast => "ne-resize",
                        WindowEdge::SouthEast => "se-resize",
                        WindowEdge::SouthWest => "sw-resize",
                        _ => "default",
                      },
                    )
                    .as_ref(),
                  );
                }
              }
              Inhibit(false)
            });
            window.connect_button_press_event(|window, event| {
              if !window.is_decorated() && window.is_resizable() && event.button() == 1 {
                if let Some(window) = window.window() {
                  let (cx, cy) = event.root();
                  let result = hit_test(&window, cx, cy);

                  // Ignore the `__Unknown` variant so the window receives the click correctly if it is not on the edges.
                  match result {
                    WindowEdge::__Unknown(_) => (),
                    _ => {
                      // FIXME: calling `window.begin_resize_drag` uses the default cursor, it should show a resizing cursor instead
                      window.begin_resize_drag(result, 1, cx as i32, cy as i32, event.time())
                    }
                  }
                }
              }

              Inhibit(false)
            });
            window.connect_touch_event(|window, event| {
              if !window.is_decorated() && window.is_resizable() {
                if let Some(window) = window.window() {
                  if let Some((cx, cy)) = event.root_coords() {
                    if let Some(device) = event.device() {
                      let result = hit_test(&window, cx, cy);

                      // Ignore the `__Unknown` variant so the window receives the click correctly if it is not on the edges.
                      match result {
                        WindowEdge::__Unknown(_) => (),
                        _ => window.begin_resize_drag_for_device(
                          result,
                          &device,
                          0,
                          cx as i32,
                          cy as i32,
                          event.time(),
                        ),
                      }
                    }
                  }
                }
              }

              Inhibit(false)
            });

            let tx_clone = event_tx.clone();
            window.connect_delete_event(move |_, _| {
              if let Err(e) = tx_clone.send(Event::WindowEvent {
                window_id: RootWindowId(id),
                event: WindowEvent::CloseRequested,
              }) {
                log::warn!("Failed to send window close event to event channel: {}", e);
              }
              Inhibit(true)
            });

            let tx_clone = event_tx.clone();
            window.connect_configure_event(move |window, event| {
              let scale_factor = window.scale_factor();

              let (x, y) = event.position();
              if let Err(e) = tx_clone.send(Event::WindowEvent {
                window_id: RootWindowId(id),
                event: WindowEvent::Moved(
                  LogicalPosition::new(x, y).to_physical(scale_factor as f64),
                ),
              }) {
                log::warn!("Failed to send window moved event to event channel: {}", e);
              }

              let (w, h) = event.size();
              if let Err(e) = tx_clone.send(Event::WindowEvent {
                window_id: RootWindowId(id),
                event: WindowEvent::Resized(
                  LogicalSize::new(w, h).to_physical(scale_factor as f64),
                ),
              }) {
                log::warn!(
                  "Failed to send window resized event to event channel: {}",
                  e
                );
              }
              false
            });

            let tx_clone = event_tx.clone();
            window.connect_focus_in_event(move |_, _| {
              if let Err(e) = tx_clone.send(Event::WindowEvent {
                window_id: RootWindowId(id),
                event: WindowEvent::Focused(true),
              }) {
                log::warn!(
                  "Failed to send window focus-in event to event channel: {}",
                  e
                );
              }
              Inhibit(false)
            });

            let tx_clone = event_tx.clone();
            window.connect_focus_out_event(move |_, _| {
              if let Err(e) = tx_clone.send(Event::WindowEvent {
                window_id: RootWindowId(id),
                event: WindowEvent::Focused(false),
              }) {
                log::warn!(
                  "Failed to send window focus-out event to event channel: {}",
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
                  device_id: DEVICE_ID,
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
            window.connect_motion_notify_event(move |window, motion| {
              if let Some(cursor) = motion.device() {
                let scale_factor = window.scale_factor();
                let (_, x, y) = cursor.window_at_position();
                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::CursorMoved {
                    position: LogicalPosition::new(x, y).to_physical(scale_factor as f64),
                    device_id: DEVICE_ID,
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
                  device_id: DEVICE_ID,
                },
              }) {
                log::warn!("Failed to send cursor left event to event channel: {}", e);
              }
              Inhibit(false)
            });

            let tx_clone = event_tx.clone();
            window.connect_button_press_event(move |_, event| {
              let button = event.button();
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
                  device_id: DEVICE_ID,
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
              let button = event.button();
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
                  device_id: DEVICE_ID,
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

            let tx_clone = event_tx.clone();
            let keyboard_handler = Rc::new(move |event_key: EventKey, element_state| {
              // if we have a modifier lets send it
              let mut mods = keyboard::get_modifiers(event_key.clone());
              if !mods.is_empty() {
                // if we release the modifier tell the world
                if ElementState::Released == element_state {
                  mods = ModifiersState::empty();
                }

                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::ModifiersChanged(mods),
                }) {
                  log::warn!(
                    "Failed to send modifiers changed event to event channel: {}",
                    e
                  );
                } else {
                  // stop here we don't want to send the key event
                  // as we emit the `ModifiersChanged`
                  return Continue(true);
                }
              }

              // todo: implement repeat?
              let event = keyboard::make_key_event(&event_key, false, None, element_state);

              if let Some(event) = event {
                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::KeyboardInput {
                    device_id: DEVICE_ID,
                    event,
                    is_synthetic: false,
                  },
                }) {
                  log::warn!("Failed to send keyboard event to event channel: {}", e);
                }
              }
              Continue(true)
            });

            let tx_clone = event_tx.clone();
            // TODO Add actual IME from system
            let ime = gtk::IMContextSimple::default();
            ime.set_client_window(window.window().as_ref());
            ime.focus_in();
            ime.connect_commit(move |_, s| {
              if let Err(e) = tx_clone.send(Event::WindowEvent {
                window_id: RootWindowId(id),
                event: WindowEvent::ReceivedImeText(s.to_string()),
              }) {
                log::warn!(
                  "Failed to send received IME text event to event channel: {}",
                  e
                );
              }
            });

            let handler = keyboard_handler.clone();
            window.connect_key_press_event(move |_, event_key| {
              handler(event_key.to_owned(), ElementState::Pressed);
              ime.filter_keypress(event_key);

              Inhibit(false)
            });

            let handler = keyboard_handler.clone();
            window.connect_key_release_event(move |_, event_key| {
              handler(event_key.to_owned(), ElementState::Released);
              Inhibit(false)
            });

            let tx_clone = event_tx.clone();
            window.connect_window_state_event(move |window, event| {
              let state = event.changed_mask();
              if state.contains(WindowState::ICONIFIED) || state.contains(WindowState::MAXIMIZED) {
                let scale_factor = window.scale_factor();

                let (x, y) = window.position();
                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::Moved(
                    LogicalPosition::new(x, y).to_physical(scale_factor as f64),
                  ),
                }) {
                  log::warn!("Failed to send window moved event to event channel: {}", e);
                }

                let (w, h) = window.size();
                if let Err(e) = tx_clone.send(Event::WindowEvent {
                  window_id: RootWindowId(id),
                  event: WindowEvent::Resized(
                    LogicalSize::new(w, h).to_physical(scale_factor as f64),
                  ),
                }) {
                  log::warn!(
                    "Failed to send window resized event to event channel: {}",
                    e
                  );
                }
              }
              Inhibit(false)
            });
          }
          WindowRequest::Redraw => {
            if let Err(e) = draw_tx.send(id) {
              log::warn!("Failed to send redraw event to event channel: {}", e);
            }

            window.queue_draw();
          }
          WindowRequest::Menu(m) => match m {
            (None, Some(menu_id)) => {
              if let Err(e) = event_tx.send(Event::MenuEvent {
                window_id: Some(RootWindowId(id)),
                menu_id,
                origin: MenuType::MenuBar,
              }) {
                log::warn!("Failed to send menu event to event channel: {}", e);
              }
            }
            (Some(MenuItem::About(_)), None) => {
              let about = AboutDialog::new();
              about.show_all();
              app_.add_window(&about);
            }
            (Some(MenuItem::Hide), None) => window.hide(),
            (Some(MenuItem::CloseWindow), None) => window.close(),
            (Some(MenuItem::Quit), None) => {
              if let Err(e) = event_tx.send(Event::LoopDestroyed) {
                log::warn!(
                  "Failed to send loop destroyed event to event channel: {}",
                  e
                );
              }
            }
            (Some(MenuItem::EnterFullScreen), None) => {
              let state = window.window().unwrap().state();
              if state.contains(WindowState::FULLSCREEN) {
                window.unfullscreen();
              } else {
                window.fullscreen();
              }
            }
            (Some(MenuItem::Minimize), None) => window.iconify(),
            _ => {}
          },
          WindowRequest::SetMenu((window_menu, accel_group, mut menubar)) => {
            if let Some(window_menu) = window_menu {
              // remove all existing elements as we overwrite
              // but we keep same menubar reference
              for i in menubar.children() {
                menubar.remove(&i);
              }
              // create all new elements
              window_menu.generate_menu(&mut menubar, &window_requests_tx_, &accel_group, id);
              // make sure all newly added elements are visible
              menubar.show_all();
            }
          }
          WindowRequest::GlobalHotKey(_hotkey_id) => {}
        }
      } else if id == WindowId::dummy() {
        match request {
          WindowRequest::GlobalHotKey(hotkey_id) => {
            if let Err(e) = event_tx.send(Event::GlobalShortcutEvent(AcceleratorId(hotkey_id))) {
              log::warn!("Failed to send global hotkey event to event channel: {}", e);
            }
          }
          WindowRequest::Menu((None, Some(menu_id))) => {
            if let Err(e) = event_tx.send(Event::MenuEvent {
              window_id: None,
              menu_id,
              origin: MenuType::ContextMenu,
            }) {
              log::warn!("Failed to send status bar event to event channel: {}", e);
            }
          }
          _ => {}
        }
      }
      Continue(true)
    });

    // Create event loop itself.
    let event_loop = Self {
      window_target: RootELW {
        p: window_target,
        _marker: std::marker::PhantomData,
      },
      user_event_tx,
      events: event_rx,
      draws: draw_rx,
    };

    Ok(event_loop)
  }

  #[inline]
  pub fn run<F>(mut self, callback: F) -> !
  where
    F: FnMut(Event<'_, T>, &RootELW<T>, &mut ControlFlow) + 'static,
  {
    let exit_code = self.run_return(callback);
    process::exit(exit_code)
  }

  /// This is the core event loop logic. It basically loops on `gtk_main_iteration` and processes one
  /// event along with that iteration. Depends on current control flow and what it should do, an
  /// event state is defined. The whole state flow chart runs like following:
  ///
  /// ```ignore
  ///                                   Poll/Wait/WaitUntil
  ///       +-------------------------------------------------------------------------+
  ///       |                                                                         |
  ///       |                   Receiving event from event channel                    |   Receiving event from draw channel
  ///       |                               +-------+                                 |   +---+
  ///       v                               v       |                                 |   v   |
  /// +----------+  Poll/Wait/WaitUntil   +------------+  Poll/Wait/WaitUntil   +-----------+ |
  /// | NewStart | ---------------------> | EventQueue | ---------------------> | DrawQueue | |
  /// +----------+                        +------------+                        +-----------+ |
  ///       |ExitWithCode                        |ExitWithCode            ExitWithCode|   |   |
  ///       +------------------------------------+------------------------------------+   +---+
  ///                                            |
  ///                                            v
  ///                                    +---------------+
  ///                                    | LoopDestroyed |
  ///                                    +---------------+
  /// ```
  ///
  /// There are a dew notibale event will sent to callback when state is transisted:
  /// - On any state moves to `LoopDestroyed`, a `LoopDestroyed` event is sent.
  /// - On `NewStart` to `EventQueue`, a `NewEvents` with corresponding `StartCause` depends on
  /// current control flow is sent.
  /// - On `EventQueue` to `DrawQueue`, a `MainEventsCleared` event is sent.
  /// - On `DrawQueue` back to `NewStart`, a `RedrawEventsCleared` event is sent.
  pub(crate) fn run_return<F>(&mut self, mut callback: F) -> i32
  where
    F: FnMut(Event<'_, T>, &RootELW<T>, &mut ControlFlow),
  {
    enum EventState {
      NewStart,
      EventQueue,
      DrawQueue,
    }

    let context = MainContext::default();
    context
      .with_thread_default(|| {
        let mut control_flow = ControlFlow::default();
        let window_target = &self.window_target;
        let events = &self.events;
        let draws = &self.draws;

        window_target.p.app.activate();

        let mut state = EventState::NewStart;
        let exit_code = loop {
          let mut blocking = false;
          match state {
            EventState::NewStart => match control_flow {
              ControlFlow::ExitWithCode(code) => {
                callback(Event::LoopDestroyed, window_target, &mut control_flow);
                break code;
              }
              ControlFlow::Wait => {
                if !events.is_empty() || !draws.is_empty() {
                  callback(
                    Event::NewEvents(StartCause::WaitCancelled {
                      start: Instant::now(),
                      requested_resume: None,
                    }),
                    window_target,
                    &mut control_flow,
                  );
                  state = EventState::EventQueue;
                } else {
                  blocking = true;
                }
              }
              ControlFlow::WaitUntil(requested_resume) => {
                let start = Instant::now();
                if start >= requested_resume {
                  callback(
                    Event::NewEvents(StartCause::ResumeTimeReached {
                      start,
                      requested_resume,
                    }),
                    window_target,
                    &mut control_flow,
                  );
                  state = EventState::EventQueue;
                } else if !events.is_empty() {
                  callback(
                    Event::NewEvents(StartCause::WaitCancelled {
                      start,
                      requested_resume: Some(requested_resume),
                    }),
                    window_target,
                    &mut control_flow,
                  );
                  state = EventState::EventQueue;
                } else {
                  blocking = true;
                }
              }
              ControlFlow::Poll => {
                callback(
                  Event::NewEvents(StartCause::Poll),
                  window_target,
                  &mut control_flow,
                );
                state = EventState::EventQueue;
              }
            },
            EventState::EventQueue => match control_flow {
              ControlFlow::ExitWithCode(code) => {
                callback(Event::LoopDestroyed, window_target, &mut control_flow);
                break (code);
              }
              _ => match events.try_recv() {
                Ok(event) => match event {
                  Event::LoopDestroyed => control_flow = ControlFlow::ExitWithCode(1),
                  _ => callback(event, window_target, &mut control_flow),
                },
                Err(_) => {
                  callback(Event::MainEventsCleared, window_target, &mut control_flow);
                  if draws.is_empty() {
                    state = EventState::NewStart;
                  } else {
                    state = EventState::DrawQueue;
                  }
                }
              },
            },
            EventState::DrawQueue => match control_flow {
              ControlFlow::ExitWithCode(code) => {
                callback(Event::LoopDestroyed, window_target, &mut control_flow);
                break code;
              }
              _ => match draws.try_recv() {
                Ok(id) => callback(
                  Event::RedrawRequested(RootWindowId(id)),
                  window_target,
                  &mut control_flow,
                ),
                Err(_) => {
                  callback(Event::RedrawEventsCleared, window_target, &mut control_flow);
                  state = EventState::NewStart;
                }
              },
            },
          }
          gtk::main_iteration_do(blocking);
        };
        exit_code
      })
      .unwrap_or(1)
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
  assert!(
    is_main_thread(),
    "Initializing the event loop outside of the main thread is a significant \
             cross-platform compatibility hazard. If you really, absolutely need to create an \
             EventLoop on a different thread, please use the `EventLoopExtUnix::{}` function.",
    suggested_method
  );
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
