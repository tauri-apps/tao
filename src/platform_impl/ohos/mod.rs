use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use keycodes::{to_location, to_logical};
use openharmony_ability::xcomponent::{Action, TouchEvent};

use openharmony_ability::{
  ime::KeyboardStatus, Configuration, Event as MainEvent, ImeEvent, InputEvent, OpenHarmonyApp,
  OpenHarmonyWaker, Rect,
};

use crate::dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use crate::error::{self};
use crate::event::{self, ElementState, Force, StartCause};
use crate::event_loop::{self, ControlFlow};
use crate::keyboard::{Key, KeyCode, KeyLocation, NativeKeyCode};
use crate::monitor;
use crate::window::{self, Fullscreen, ResizeDirection, Theme, WindowSizeConstraints};

mod keycodes;

pub(crate) use crate::icon::NoIcon as PlatformIcon;

static HAS_FOCUS: AtomicBool = AtomicBool::new(true);

struct PeekableReceiver<T> {
  recv: mpsc::Receiver<T>,
  first: Option<T>,
}

impl<T> PeekableReceiver<T> {
  pub fn from_recv(recv: mpsc::Receiver<T>) -> Self {
    Self { recv, first: None }
  }

  pub fn try_recv(&mut self) -> Result<T, mpsc::TryRecvError> {
    if let Some(first) = self.first.take() {
      return Ok(first);
    }
    self.recv.try_recv()
  }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct KeyEventExtra {}

pub struct EventLoop<T: 'static> {
  pub(crate) openharmony_app: OpenHarmonyApp,
  window_target: event_loop::EventLoopWindowTarget<T>,
  cause: StartCause,
  user_events_sender: mpsc::Sender<T>,
  user_events_receiver: PeekableReceiver<T>,
  event_loop: RefCell<Option<Box<dyn FnMut(event::Event<T>)>>>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct PlatformSpecificEventLoopAttributes {
  pub(crate) openharmony_app: Option<OpenHarmonyApp>,
}

impl Default for PlatformSpecificEventLoopAttributes {
  fn default() -> Self {
    Self {
      openharmony_app: Default::default(),
    }
  }
}

impl<T: 'static> EventLoop<T> {
  pub(crate) fn new(attributes: &PlatformSpecificEventLoopAttributes) -> Self {
    let (user_events_sender, user_events_receiver) = mpsc::channel();

    let openharmony_app = attributes.openharmony_app.as_ref().expect(
      "An `OpenHarmonyApp` as passed to lib is required to create an `EventLoop` on \
             OpenHarmony or HarmonyNext",
    );

    Self {
      openharmony_app: openharmony_app.clone(),
      window_target: event_loop::EventLoopWindowTarget {
        p: EventLoopWindowTarget {
          app: openharmony_app.clone(),
          control_flow: Cell::new(ControlFlow::default()),
          exit: Cell::new(false),
          _marker: PhantomData,
        },
        _marker: PhantomData,
      },
      cause: StartCause::Init,
      user_events_sender,
      user_events_receiver: PeekableReceiver::from_recv(user_events_receiver),
      event_loop: RefCell::new(None),
    }
  }

  pub(crate) fn window_target(&self) -> &event_loop::EventLoopWindowTarget<T> {
    &self.window_target
  }

  // TODO: For input event, we need some real examples to test it
  fn handle_input_event(&self, event: &InputEvent) {
    match event {
      InputEvent::TouchEvent(motion_event) => {
        let window_id = window::WindowId(WindowId);
        let device_id = event::DeviceId(DeviceId(motion_event.device_id as _));
        let action = motion_event.event_type;

        let phase = match motion_event.event_type {
          TouchEvent::Down => Some(event::TouchPhase::Started),
          TouchEvent::Up => Some(event::TouchPhase::Ended),
          TouchEvent::Move => Some(event::TouchPhase::Moved),
          TouchEvent::Cancel => Some(event::TouchPhase::Cancelled),
          _ => {
            None // TODO mouse events
          }
        };

        if let Some(phase) = phase {
          for pointer in motion_event.touch_points.iter() {
            let position = PhysicalPosition {
              x: pointer.x as _,
              y: pointer.y as _,
            };
            trace!(
              "Input event {device_id:?}, {action:?}, loc={position:?}, \
                                 pointer={pointer:?}"
            );

            let event = event::Event::WindowEvent {
              window_id,
              event: event::WindowEvent::Touch(event::Touch {
                device_id,
                phase,
                location: position,
                id: pointer.id as u64,
                force: Some(Force::Normalized(pointer.force as f64)),
              }),
            };
            if let Some(ref mut h) = *self.event_loop.borrow_mut() {
              h(event);
            }
          }
        }
      }
      InputEvent::KeyEvent(key) => {
        match key.code {
          keycode => {
            let state = match key.action {
              Action::Down => event::ElementState::Pressed,
              Action::Up => event::ElementState::Released,
              _ => event::ElementState::Released,
            };

            let native = NativeKeyCode::Ohos(keycode.into());
            let physical_key = KeyCode::Unidentified(native);
            let logical_key = to_logical(keycode);

            let event = event::Event::WindowEvent {
              window_id: window::WindowId(WindowId),
              event: event::WindowEvent::KeyboardInput {
                device_id: event::DeviceId(DeviceId(key.device_id as _)),
                event: event::KeyEvent {
                  state,
                  physical_key,
                  logical_key,
                  location: to_location(keycode),
                  // TODO
                  repeat: false,
                  text: None,
                  platform_specific: KeyEventExtra {},
                },
                is_synthetic: false,
              },
            };
            if let Some(ref mut h) = *self.event_loop.borrow_mut() {
              h(event);
            }
          }
        }
      }
      InputEvent::ImeEvent(data) => match data {
        ImeEvent::TextInputEvent(s) => {
          if let Some(ref mut h) = *self.event_loop.borrow_mut() {
            h(event::Event::WindowEvent {
              window_id: window::WindowId(WindowId),
              event: event::WindowEvent::ReceivedImeText(s.text.clone()),
            })
          }
        }
        ImeEvent::BackspaceEvent(_) => {
          if let Some(ref mut h) = *self.event_loop.borrow_mut() {
            // Mock keyboard input event
            [ElementState::Pressed, ElementState::Released].map(|state| {
              h(event::Event::WindowEvent {
                window_id: window::WindowId(WindowId),
                event: event::WindowEvent::KeyboardInput {
                  device_id: event::DeviceId(DeviceId(0)),
                  event: event::KeyEvent {
                    state,
                    logical_key: Key::Backspace,
                    physical_key: KeyCode::Backspace,
                    platform_specific: KeyEventExtra {},
                    repeat: false,
                    location: KeyLocation::Standard,
                    text: None,
                  },
                  is_synthetic: false,
                },
              });
            });
          }
        }

        ImeEvent::ImeStatusEvent(s) => match s {
          KeyboardStatus::Hide => {
            if let Some(ref mut h) = *self.event_loop.borrow_mut() {
              // Mock keyboard input event that make sure egui can receive the event and trigger onblur event
              [ElementState::Pressed, ElementState::Released].map(|state| {
                h(event::Event::WindowEvent {
                  window_id: window::WindowId(WindowId),
                  event: event::WindowEvent::KeyboardInput {
                    device_id: event::DeviceId(DeviceId(0)),
                    event: event::KeyEvent {
                      state,
                      logical_key: Key::Enter,
                      physical_key: KeyCode::Enter,
                      platform_specific: KeyEventExtra {},
                      repeat: false,
                      location: KeyLocation::Standard,
                      text: None,
                    },
                    is_synthetic: false,
                  },
                });
              });
            }
          }
          _ => {
            warn!("Unknown openharmony_ability ime status event {s:?}")
          }
        },
      },
      _ => {
        warn!("Unknown openharmony_ability input event {event:?}")
      }
    }
  }

  pub fn run<F>(self, event_handler: F) -> ()
  where
    F: FnMut(event::Event<T>, &event_loop::EventLoopWindowTarget<T>, &mut ControlFlow),
  {
    let event_looper = Box::leak(Box::new(self));
    event_looper.run_return(event_handler);
  }

  pub fn run_return<F>(&mut self, mut event_handle: F) -> i32
  where
    F: FnMut(event::Event<T>, &event_loop::EventLoopWindowTarget<T>, &mut ControlFlow),
  {
    let mut control_flow = ControlFlow::default();
    let target = &self.window_target;

    {
      let handle = unsafe {
        std::mem::transmute::<Box<dyn FnMut(event::Event<T>)>, Box<dyn FnMut(event::Event<T>)>>(
          Box::new(move |e| {
            event_handle(e, &target, &mut control_flow);
            // We need to dispatch it after every event callbacks.
            event_handle(event::Event::MainEventsCleared, &target, &mut control_flow);
          }),
        )
      };
      self.event_loop.replace(Some(handle));
    }

    self.openharmony_app.clone().run_loop(|event| {
      match event {
        MainEvent::SurfaceCreate { .. } => {
          if let Some(ref mut h) = *self.event_loop.borrow_mut() {
            h(event::Event::NewEvents(StartCause::Init));
            h(event::Event::Resumed);
          }
        }
        MainEvent::SurfaceDestroy { .. } => {
          if let Some(ref mut h) = *self.event_loop.borrow_mut() {
            h(event::Event::Suspended);
          }
        }
        MainEvent::WindowResize { .. } => {
          let win = self.openharmony_app.native_window();
          let size = if let Some(win) = win {
            PhysicalSize::new(win.width() as _, win.height() as _)
          } else {
            PhysicalSize::new(0, 0)
          };
          let event = event::Event::WindowEvent {
            window_id: window::WindowId(WindowId),
            event: event::WindowEvent::Resized(size),
          };

          if let Some(ref mut h) = *self.event_loop.borrow_mut() {
            h(event);
          }
        }
        MainEvent::WindowRedraw { .. } => {
          let event = event::Event::RedrawRequested(window::WindowId(WindowId));

          if let Some(ref mut h) = *self.event_loop.borrow_mut() {
            h(event);
          }
        }
        MainEvent::ContentRectChange { .. } => {
          warn!("TODO: find a way to notify application of content rect change");
        }
        MainEvent::GainedFocus => {
          HAS_FOCUS.store(true, Ordering::Relaxed);

          if let Some(ref mut h) = *self.event_loop.borrow_mut() {
            h(event::Event::WindowEvent {
              window_id: window::WindowId(WindowId),
              event: event::WindowEvent::Focused(true),
            });
          }
        }
        MainEvent::LostFocus => {
          HAS_FOCUS.store(false, Ordering::Relaxed);

          if let Some(ref mut h) = *self.event_loop.borrow_mut() {
            h(event::Event::WindowEvent {
              window_id: window::WindowId(WindowId),
              event: event::WindowEvent::Focused(true),
            });
          }
        }
        MainEvent::ConfigChanged { .. } => {
          let win = self.openharmony_app.native_window();
          if let Some(win) = win {
            let scale = self.openharmony_app.scale();
            let width = win.width();
            let height = win.height();
            let mut size = PhysicalSize::new(width as _, height as _);
            let event = event::Event::WindowEvent {
              window_id: window::WindowId(WindowId),
              event: event::WindowEvent::ScaleFactorChanged {
                new_inner_size: &mut size,
                scale_factor: scale as _,
              },
            };

            if let Some(ref mut h) = *self.event_loop.borrow_mut() {
              h(event);
            }
          }
        }
        MainEvent::Start => {
          // XXX: how to forward this state to applications?
          warn!("TODO: forward onStart notification to application");
        }
        MainEvent::Resume { .. } => {
          if let Some(ref mut h) = *self.event_loop.borrow_mut() {
            h(event::Event::Resumed);
          }
        }
        MainEvent::SaveState { .. } => {
          // XXX: how to forward this state to applications?
          // XXX: also how do we expose state restoration to apps?
          warn!("TODO: forward saveState notification to application");
        }
        MainEvent::Pause => {
          debug!("App Paused - stopped running");
          // TODO: This is incorrect - will be solved in https://github.com/rust-windowing/winit/pull/3897
          // self.running = false;
        }
        MainEvent::WindowDestroy => {
          if let Some(ref mut h) = *self.event_loop.borrow_mut() {
            let e = event::Event::WindowEvent {
              window_id: window::WindowId(WindowId),
              event: event::WindowEvent::CloseRequested,
            };
            h(e);
          }
        }
        MainEvent::Destroy => {
          // XXX: maybe exit mainloop to drop things before being
          // killed by the OS?
          warn!("TODO: forward onDestroy notification to application");
        }
        MainEvent::Input(input_event) => {
          self.handle_input_event(&input_event);
        }
        MainEvent::UserEvent { .. } => {
          if let Some(ref mut h) = *self.event_loop.borrow_mut() {
            if let Ok(event) = self.user_events_receiver.try_recv() {
              let event = event::Event::UserEvent(event);
              h(event);
            }
          }
        }
        unknown => {
          trace!("Unknown MainEvent {unknown:?} (ignored)");
        }
      };

      if self.window_target.p.exit.get() {
        if let Some(ref mut h) = *self.event_loop.borrow_mut() {
          h(event::Event::LoopDestroyed);
          self.openharmony_app.exit(0);
        }
      }
    });
    0
  }

  pub fn create_proxy(&self) -> EventLoopProxy<T> {
    EventLoopProxy {
      user_events_sender: self.user_events_sender.clone(),
      waker: self.openharmony_app.create_waker(),
    }
  }
}

pub struct EventLoopProxy<T: 'static> {
  user_events_sender: mpsc::Sender<T>,
  waker: OpenHarmonyWaker,
}

impl<T: 'static> EventLoopProxy<T> {
  pub fn send_event(&self, event: T) -> Result<(), event_loop::EventLoopClosed<T>> {
    self
      .user_events_sender
      .send(event)
      .map_err(|err| event_loop::EventLoopClosed(err.0))?;
    self.waker.wake();
    Ok(())
  }
}

impl<T: 'static> Clone for EventLoopProxy<T> {
  fn clone(&self) -> Self {
    EventLoopProxy {
      user_events_sender: self.user_events_sender.clone(),
      waker: self.waker.clone(),
    }
  }
}

#[derive(Clone)]
pub struct EventLoopWindowTarget<T: 'static> {
  pub(crate) app: OpenHarmonyApp,
  control_flow: Cell<ControlFlow>,
  exit: Cell<bool>,
  _marker: std::marker::PhantomData<T>,
}

impl<T: 'static> EventLoopWindowTarget<T> {
  pub fn available_monitors(&self) -> VecDeque<MonitorHandle> {
    let mut v = VecDeque::with_capacity(1);
    v.push_back(MonitorHandle::new(self.app.clone()));
    v
  }

  pub fn primary_monitor(&self) -> Option<monitor::MonitorHandle> {
    Some(monitor::MonitorHandle {
      inner: MonitorHandle::new(self.app.clone()),
    })
  }

  #[inline]
  pub fn monitor_from_point(&self, _x: f64, _y: f64) -> Option<MonitorHandle> {
    warn!("`Window::monitor_from_point` is ignored on OpenHarmony");
    return None;
  }

  #[cfg(feature = "rwh_05")]
  #[inline]
  pub fn raw_display_handle_rwh_05(&self) -> rwh_05::RawDisplayHandle {
    unreachable!("rwh_05 is not supported on OpenHarmony");
  }

  #[cfg(feature = "rwh_06")]
  #[inline]
  pub fn raw_display_handle_rwh_06(&self) -> Result<rwh_06::RawDisplayHandle, rwh_06::HandleError> {
    Ok(rwh_06::RawDisplayHandle::Ohos(
      rwh_06::OhosDisplayHandle::new(),
    ))
  }

  pub fn cursor_position(&self) -> Result<PhysicalPosition<f64>, error::ExternalError> {
    debug!("`EventLoopWindowTarget::cursor_position` is ignored on OpenHarmony");
    Ok((0, 0).into())
  }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct WindowId;

impl WindowId {
  pub const fn dummy() -> Self {
    WindowId
  }
}

impl From<WindowId> for u64 {
  fn from(_: WindowId) -> Self {
    0
  }
}

impl From<u64> for WindowId {
  fn from(_: u64) -> Self {
    Self
  }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeviceId(i32);

impl DeviceId {
  pub const fn dummy() -> Self {
    DeviceId(0)
  }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlatformSpecificWindowBuilderAttributes;

pub(crate) struct Window {
  app: OpenHarmonyApp,
}

impl Window {
  pub(crate) fn new<T: 'static>(
    el: &EventLoopWindowTarget<T>,
    _window_attrs: window::WindowAttributes,
    _: PlatformSpecificWindowBuilderAttributes,
  ) -> Result<Self, error::OsError> {
    // FIXME this ignores requested window attributes

    Ok(Self {
      app: el.app.clone(),
    })
  }

  pub fn request_redraw(&self) {}

  #[inline]
  pub fn monitor_from_point(&self, _x: f64, _y: f64) -> Option<monitor::MonitorHandle> {
    warn!("`Window::monitor_from_point` is ignored on OpenHarmony");
    return None;
  }

  pub fn id(&self) -> WindowId {
    WindowId
  }

  pub fn scale_factor(&self) -> f64 {
    self.app.scale() as f64
  }

  pub fn available_monitors(&self) -> VecDeque<MonitorHandle> {
    let mut v = VecDeque::with_capacity(1);
    v.push_back(MonitorHandle::new(self.app.clone()));
    v
  }

  pub fn inner_position(&self) -> Result<PhysicalPosition<i32>, error::NotSupportedError> {
    Err(error::NotSupportedError::new())
  }

  pub fn inner_size(&self) -> PhysicalSize<u32> {
    self.outer_size()
  }

  pub fn set_inner_size(&self, _size: Size) {
    warn!("Cannot set window size on OpenHarmony");
  }
  pub fn set_inner_size_constraints(&self, _: WindowSizeConstraints) {}

  pub fn outer_position(&self) -> Result<PhysicalPosition<i32>, error::NotSupportedError> {
    Err(error::NotSupportedError::new())
  }

  pub fn set_outer_position(&self, _position: Position) {
    // no effect
  }

  pub fn outer_size(&self) -> PhysicalSize<u32> {
    MonitorHandle::new(self.app.clone()).size()
  }

  pub fn set_min_inner_size(&self, _: Option<Size>) {}

  pub fn set_max_inner_size(&self, _: Option<Size>) {}

  pub fn set_title(&self, _title: &str) {}

  pub fn set_visible(&self, _visibility: bool) {}

  pub fn set_focus(&self) {
    //FIXME: implementation goes here
    warn!("set_focus not yet implemented on OpenHarmony");
  }

  pub fn set_focusable(&self, _focusable: bool) {
    warn!("set_focusable not yet implemented on OpenHarmony");
  }

  pub fn is_focused(&self) -> bool {
    log::warn!("`Window::is_focused` is ignored on OpenHarmony");
    false
  }

  pub fn is_always_on_top(&self) -> bool {
    log::warn!("`Window::is_always_on_top` is ignored on OpenHarmony");
    false
  }

  pub fn set_resizable(&self, _resizeable: bool) {
    warn!("`Window::set_resizable` is ignored on OpenHarmony")
  }

  pub fn set_minimizable(&self, _minimizable: bool) {
    warn!("`Window::set_minimizable` is ignored on OpenHarmony")
  }

  pub fn set_maximizable(&self, _maximizable: bool) {
    warn!("`Window::set_maximizable` is ignored on OpenHarmony")
  }

  pub fn set_closable(&self, _closable: bool) {
    warn!("`Window::set_closable` is ignored on OpenHarmony")
  }

  pub fn set_minimized(&self, _minimized: bool) {}

  pub fn is_minimized(&self) -> bool {
    false
  }

  pub fn set_maximized(&self, _maximized: bool) {}

  pub fn is_maximized(&self) -> bool {
    false
  }

  pub fn set_fullscreen(&self, _monitor: Option<Fullscreen>) {
    warn!("Cannot set fullscreen on OpenHarmony");
  }

  pub fn fullscreen(&self) -> Option<Fullscreen> {
    None
  }

  pub fn set_decorations(&self, _decorations: bool) {}
  pub fn set_always_on_bottom(&self, _always_on_bottom: bool) {}

  pub fn set_always_on_top(&self, _always_on_top: bool) {}
  pub fn set_ime_position(&self, _position: Position) {}

  pub fn is_decorated(&self) -> bool {
    true
  }

  pub fn is_visible(&self) -> bool {
    log::warn!("`Window::is_visible` is ignored on OpenHarmony");
    false
  }

  pub fn is_resizable(&self) -> bool {
    warn!("`Window::is_resizable` is ignored on OpenHarmony");
    false
  }

  pub fn is_minimizable(&self) -> bool {
    warn!("`Window::is_minimizable` is ignored on OpenHarmony");
    false
  }

  pub fn is_maximizable(&self) -> bool {
    warn!("`Window::is_maximizable` is ignored on OpenHarmony");
    false
  }

  pub fn is_closable(&self) -> bool {
    warn!("`Window::is_closable` is ignored on OpenHarmony");
    false
  }

  pub fn set_window_icon(&self, _window_icon: Option<crate::icon::Icon>) {}

  pub fn set_cursor_icon(&self, _: window::CursorIcon) {}
  pub fn set_cursor_grab(&self, _: bool) -> Result<(), error::ExternalError> {
    Err(error::ExternalError::NotSupported(
      error::NotSupportedError::new(),
    ))
  }

  pub fn request_user_attention(&self, _request_type: Option<window::UserAttentionType>) {}

  pub fn set_cursor_position(&self, _: Position) -> Result<(), error::ExternalError> {
    Err(error::ExternalError::NotSupported(
      error::NotSupportedError::new(),
    ))
  }

  pub fn cursor_position(&self) -> Result<PhysicalPosition<f64>, error::ExternalError> {
    debug!("`Window::cursor_position` is ignored on OpenHarmony");
    Ok((0, 0).into())
  }

  pub fn set_ignore_cursor_events(&self, _ignore: bool) -> Result<(), error::ExternalError> {
    Err(error::ExternalError::NotSupported(
      error::NotSupportedError::new(),
    ))
  }

  pub fn set_cursor_visible(&self, _: bool) {}
  pub fn drag_window(&self) -> Result<(), error::ExternalError> {
    Err(error::ExternalError::NotSupported(
      error::NotSupportedError::new(),
    ))
  }

  pub fn drag_resize_window(
    &self,
    _direction: ResizeDirection,
  ) -> Result<(), error::ExternalError> {
    Err(error::ExternalError::NotSupported(
      error::NotSupportedError::new(),
    ))
  }

  pub fn set_background_color(&self, _color: Option<crate::window::RGBA>) {}

  pub fn theme(&self) -> Theme {
    Theme::Light
  }

  pub fn title(&self) -> String {
    String::new()
  }

  #[cfg(feature = "rwh_04")]
  pub fn raw_window_handle_rwh_04(&self) -> rwh_04::RawWindowHandle {
    unreachable!("rwh_04 is not supported on OpenHarmony");
  }

  #[cfg(feature = "rwh_05")]
  pub fn raw_window_handle_rwh_05(&self) -> rwh_05::RawWindowHandle {
    unreachable!("rwh_05 is not supported on OpenHarmony");
  }

  #[cfg(feature = "rwh_05")]
  pub fn raw_display_handle_rwh_05(&self) -> rwh_05::RawDisplayHandle {
    unreachable!("rwh_05 is not supported on OpenHarmony");
  }

  #[cfg(feature = "rwh_06")]
  // Allow the usage of HasRawWindowHandle inside this function
  #[allow(deprecated)]
  pub fn raw_window_handle_rwh_06(&self) -> Result<rwh_06::RawWindowHandle, rwh_06::HandleError> {
    if let Some(native_window) = self.app.native_window().as_ref() {
      if let Some(win) = native_window.raw_window_handle() {
        return Ok(win);
      }
      Err(rwh_06::HandleError::Unavailable)
    } else {
      Err(rwh_06::HandleError::Unavailable)
    }
  }

  #[cfg(feature = "rwh_06")]
  pub fn raw_display_handle_rwh_06(&self) -> Result<rwh_06::RawDisplayHandle, rwh_06::HandleError> {
    Ok(rwh_06::RawDisplayHandle::Ohos(
      rwh_06::OhosDisplayHandle::new(),
    ))
  }

  pub fn config(&self) -> Configuration {
    self.app.config()
  }

  pub fn content_rect(&self) -> Rect {
    self.app.content_rect()
  }

  pub fn current_monitor(&self) -> Option<monitor::MonitorHandle> {
    Some(monitor::MonitorHandle {
      inner: MonitorHandle::new(self.app.clone()),
    })
  }

  pub fn primary_monitor(&self) -> Option<monitor::MonitorHandle> {
    Some(monitor::MonitorHandle {
      inner: MonitorHandle::new(self.app.clone()),
    })
  }
}

#[derive(Default, Clone, Debug)]
pub struct OsError;

use std::fmt::{self, Display, Formatter};
impl Display for OsError {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), fmt::Error> {
    write!(fmt, "OpenHarmony OS Error")
  }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MonitorHandle {
  app: OpenHarmonyApp,
}

impl MonitorHandle {
  pub(crate) fn new(app: OpenHarmonyApp) -> Self {
    Self { app }
  }

  pub fn name(&self) -> Option<String> {
    Some("OpenHarmony Device".to_owned())
  }

  pub fn size(&self) -> PhysicalSize<u32> {
    if let Some(native_window) = self.app.native_window() {
      PhysicalSize::new(native_window.width() as _, native_window.height() as _)
    } else {
      PhysicalSize::new(0, 0)
    }
  }

  pub fn position(&self) -> PhysicalPosition<i32> {
    (0, 0).into()
  }

  pub fn scale_factor(&self) -> f64 {
    self.app.scale() as f64
  }

  pub fn video_modes(&self) -> impl Iterator<Item = monitor::VideoMode> {
    let size = self.size().into();
    // FIXME this is not the real refresh rate
    // (it is guaranteed to support 32 bit color though)
    std::iter::once(monitor::VideoMode {
      video_mode: VideoMode {
        size,
        bit_depth: 32,
        refresh_rate: 60,
        monitor: self.clone(),
      },
    })
  }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct VideoMode {
  size: (u32, u32),
  bit_depth: u16,
  refresh_rate: u16,
  monitor: MonitorHandle,
}

impl VideoMode {
  pub fn size(&self) -> PhysicalSize<u32> {
    self.size.into()
  }

  pub fn bit_depth(&self) -> u16 {
    self.bit_depth
  }

  pub fn refresh_rate(&self) -> u16 {
    self.refresh_rate
  }

  pub fn monitor(&self) -> monitor::MonitorHandle {
    monitor::MonitorHandle {
      inner: self.monitor.clone(),
    }
  }
}
pub fn keycode_to_scancode(_code: KeyCode) -> Option<u32> {
  None
}

pub fn keycode_from_scancode(_scancode: u32) -> KeyCode {
  KeyCode::Unidentified(NativeKeyCode::Unidentified)
}
