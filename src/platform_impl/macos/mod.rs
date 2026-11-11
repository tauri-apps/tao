// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

mod app;
mod app_delegate;
mod app_state;
mod badge;
mod dock;
mod event;
mod event_loop;
mod ffi;
mod icon;
mod keycode;
mod monitor;
mod observer;
mod progress_bar;
mod util;
mod view;
mod window;
mod window_delegate;

use std::fmt;

pub(crate) use self::event_loop::PlatformSpecificEventLoopAttributes;
pub use self::{
  app_delegate::get_aux_state_mut,
  event::KeyEventExtra,
  event_loop::{EventLoop, EventLoopWindowTarget, Proxy as EventLoopProxy},
  keycode::{keycode_from_scancode, keycode_to_scancode},
  monitor::{MonitorHandle, VideoMode},
  progress_bar::set_progress_indicator,
  window::{Id as WindowId, Window},
  window_delegate::{Parent, PlatformSpecificWindowBuilderAttributes},
};
use crate::event::DeviceId as RootDeviceId;
pub(crate) use badge::set_badge_label;
pub(crate) use dock::set_dock_visibility;
pub(crate) use icon::PlatformIcon;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId;

impl DeviceId {
  pub unsafe fn dummy() -> Self {
    DeviceId
  }
}

// Constant device ID; to be removed when if backend is updated to report real device IDs.
pub(crate) const DEVICE_ID: RootDeviceId = RootDeviceId(DeviceId);

#[non_exhaustive]
#[derive(Debug)]
pub enum OsError {
  CGError(core_graphics::base::CGError),
}

impl fmt::Display for OsError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      OsError::CGError(e) => f.pad(&format!("CGError {e}")),
    }
  }
}
