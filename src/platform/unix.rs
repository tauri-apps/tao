// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(any(
  target_os = "linux",
  target_os = "dragonfly",
  target_os = "freebsd",
  target_os = "netbsd",
  target_os = "openbsd"
))]

use std::{os::raw::c_int, sync::Arc};

// XConnection utilities
#[doc(hidden)]
pub use crate::platform_impl::x11;

pub use crate::platform_impl::{hit_test, EventLoop as UnixEventLoop};
use crate::{
  event_loop::{EventLoop, EventLoopWindowTarget},
  platform_impl::{x11::xdisplay::XError, Parent},
  window::{Window, WindowBuilder},
};

use self::x11::xdisplay::XConnection;

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

  /// Whether to enable or disable the internal draw for transparent window.
  ///
  /// When tranparent attribute is enabled, we will call `connect_draw` and draw a transparent background.
  /// For anyone who wants to draw the background themselves, set this to `false`.
  /// Default is `true`.
  fn with_transparent_draw(self, draw: bool) -> WindowBuilder;

  /// Whether to enable or disable the double buffered rendering of the window.
  ///
  /// Default is `true`.
  fn with_double_buffered(self, double_buffered: bool) -> WindowBuilder;

  /// Whether to enable the rgba visual for the window.
  ///
  /// Default is `false` but is always `true` if [`WindowAttributes::transparent`](crate::window::WindowAttributes::transparent) is `true`
  fn with_rgba_visual(self, rgba_visual: bool) -> WindowBuilder;

  /// Wether to set this window as app paintable
  ///
  /// <https://docs.gtk.org/gtk3/method.Widget.set_app_paintable.html>
  ///
  /// Default is `false` but is always `true` if [`WindowAttributes::transparent`](crate::window::WindowAttributes::transparent) is `true`
  fn with_app_paintable(self, app_paintable: bool) -> WindowBuilder;

  /// Whether to set cursor moved event. Cursor event is suited for native GUI frameworks and
  /// games. But it can block gtk's own pipeline occasionally. Turn this off can help Gtk looks
  /// smoother.
  ///
  /// Default is `true`.
  fn with_cursor_moved_event(self, cursor_moved: bool) -> WindowBuilder;
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

  fn with_transparent_draw(mut self, draw: bool) -> WindowBuilder {
    self.platform_specific.auto_transparent = draw;
    self
  }

  fn with_double_buffered(mut self, double_buffered: bool) -> WindowBuilder {
    self.platform_specific.double_buffered = double_buffered;
    self
  }

  fn with_rgba_visual(mut self, rgba_visual: bool) -> WindowBuilder {
    self.platform_specific.rgba_visual = rgba_visual;
    self
  }

  fn with_app_paintable(mut self, app_paintable: bool) -> WindowBuilder {
    self.platform_specific.app_paintable = app_paintable;
    self
  }

  fn with_cursor_moved_event(mut self, cursor_moved: bool) -> WindowBuilder {
    self.platform_specific.cursor_moved = cursor_moved;
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

  fn xlib_xconnection(&self) -> Option<Arc<XConnection>>;

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

  #[inline]
  fn xlib_xconnection(&self) -> Option<Arc<XConnection>> {
    if self.is_x11() {
      if let Ok(xconn) = XConnection::new(Some(x_error_callback)) {
        Some(Arc::new(xconn))
      } else {
        None
      }
    } else {
      None
    }
  }

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

unsafe extern "C" fn x_error_callback(
  _display: *mut x11::ffi::Display,
  event: *mut x11::ffi::XErrorEvent,
) -> c_int {
  let error = XError {
    // TODO get the error text as description
    description: String::new(),
    error_code: (*event).error_code,
    request_code: (*event).request_code,
    minor_code: (*event).minor_code,
  };

  error!("X11 error: {:#?}", error);

  // Fun fact: this return value is completely ignored.
  0
}
