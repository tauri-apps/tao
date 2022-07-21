// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(any(
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

pub use crate::platform_impl::{hit_test, EventLoop as UnixEventLoop};
use crate::{
  event_loop::{EventLoop, EventLoopWindowTarget},
  platform_impl::Parent,
  window::{Window, WindowBuilder},
};

/// Additional methods on `Window` that are specific to Unix.
pub trait WindowExtUnix {
  /// Returns the `ApplicatonWindow` from gtk crate that is used by this window.
  fn gtk_window(&self) -> &gtk::ApplicationWindow;

  /// Whether to show the window icon in the taskbar or not.
  fn set_skip_taskbar(&self, skip: bool);
}

impl WindowExtUnix for Window {
  fn gtk_window(&self) -> &gtk::ApplicationWindow {
    &self.window.window
  }

  fn set_skip_taskbar(&self, skip: bool) {
    self.window.set_skip_taskbar(skip);
  }
}

pub trait WindowBuilderExtUnix {
  /// Whether to create the window icon with the taskbar icon or not.
  fn with_skip_taskbar(self, skip: bool) -> WindowBuilder;
  /// Set this window as a transient dialog for `parent`
  /// <https://gtk-rs.org/gtk3-rs/stable/latest/docs/gdk/struct.Window.html#method.set_transient_for>
  fn with_transient_for(self, parent: gtk::ApplicationWindow) -> WindowBuilder;

  /// Receive draw event of the window. Set this to `true` if you want to receive draw events of the window.
  /// This will overwrite transparent attributes.
  /// Default is `true`.
  fn with_draw_event(self, filter: bool) -> WindowBuilder;
}

impl WindowBuilderExtUnix for WindowBuilder {
  fn with_skip_taskbar(mut self, skip: bool) -> WindowBuilder {
    self.platform_specific.skip_taskbar = skip;
    self
  }

  fn with_transient_for(mut self, parent: gtk::ApplicationWindow) -> WindowBuilder {
    self.platform_specific.parent = Parent::ChildOf(parent);
    self
  }

  fn with_draw_event(mut self, filter: bool) -> WindowBuilder {
    self.platform_specific.draw_event = filter;
    self
  }
}

/// Additional methods on `EventLoop` that are specific to Unix.
pub trait EventLoopExtUnix {
  /// Builds a new `EventLoop` on any thread.
  ///
  /// This method bypasses the cross-platform compatibility requirement
  /// that `EventLoop` be created on the main thread.
  fn new_any_thread() -> Self
  where
    Self: Sized;
}

fn wrap_ev<T>(event_loop: UnixEventLoop<T>) -> EventLoop<T> {
  EventLoop {
    event_loop,
    _marker: std::marker::PhantomData,
  }
}

impl<T> EventLoopExtUnix for EventLoop<T> {
  #[inline]
  fn new_any_thread() -> Self {
    wrap_ev(UnixEventLoop::new_any_thread())
  }
}

/// Additional methods on `EventLoopWindowTarget` that are specific to Unix.
pub trait EventLoopWindowTargetExtUnix {
  /// True if the `EventLoopWindowTarget` uses Wayland.
  fn is_wayland(&self) -> bool;

  /// True if the `EventLoopWindowTarget` uses X11.
  fn is_x11(&self) -> bool;

  // fn xlib_xconnection(&self) -> Option<Arc<XConnection>>;

  // /// Returns a pointer to the `wl_display` object of wayland that is used by this
  // /// `EventLoopWindowTarget`.
  // ///
  // /// Returns `None` if the `EventLoop` doesn't use wayland (if it uses xlib for example).
  // ///
  // /// The pointer will become invalid when the winit `EventLoop` is destroyed.
  // fn wayland_display(&self) -> Option<*mut raw::c_void>;
}

impl<T> EventLoopWindowTargetExtUnix for EventLoopWindowTarget<T> {
  #[inline]
  fn is_wayland(&self) -> bool {
      self.p.is_wayland()
  }

  #[inline]
  fn is_x11(&self) -> bool {
      !self.p.is_wayland()
  }

  // #[inline]
  // fn xlib_xconnection(&self) -> Option<Arc<XConnection>> {
  //     match self.p {
  //         LinuxEventLoopWindowTarget::X(ref e) => Some(e.x_connection().clone()),
  //         #[cfg(feature = "wayland")]
  //         _ => None,
  //     }
  // }

  // #[inline]
  // fn wayland_display(&self) -> Option<*mut raw::c_void> {
  //     match self.p {
  //         LinuxEventLoopWindowTarget::Wayland(ref p) => {
  //             Some(p.display().get_display_ptr() as *mut _)
  //         }
  //         #[cfg(feature = "x11")]
  //         _ => None,
  //     }
  // }
}