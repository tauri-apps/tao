// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(target_os = "android")]

pub mod prelude {
  pub use crate::platform_impl::ndk_glue::*;
  pub use tao_macros::{android_fn, generate_package_name};
}
use crate::{
  error::OsError,
  event_loop::{EventLoop, EventLoopWindowTarget},
  platform_impl::ndk_glue::Rect,
  window::{Window, WindowBuilder},
};

pub use crate::platform_impl::EmbeddedActivity;
use ndk::configuration::Configuration;

/// Additional methods on `EventLoop` that are specific to Android.
pub trait EventLoopExtAndroid {}

impl<T> EventLoopExtAndroid for EventLoop<T> {}

/// Additional methods on `EventLoopWindowTarget` that are specific to Android.
pub trait EventLoopWindowTargetExtAndroid {}

/// Additional methods on `Window` that are specific to Android.
pub trait WindowExtAndroid {
  fn content_rect(&self) -> Rect;

  fn config(&self) -> Configuration;

  fn activity_name(&self) -> String;

  /// Starts an activity of the given class in this window's task, to host another webview.
  ///
  /// Unlike a window, which gets a task of its own, the activity is stacked on top of the window's activity.
  /// [Activity embedding](https://developer.android.com/develop/ui/views/layout/activity-embedding) rules
  /// can show the two side by side on large screens.
  ///
  /// The activity must be declared in `AndroidManifest.xml` with a launch mode that allows it
  /// to be started on top of the window's activity, such as `standard`.
  /// It is finished when the returned [`EmbeddedActivity`] is dropped.
  fn create_embedded_activity(&self, activity_name: &str) -> Result<EmbeddedActivity, OsError>;
}

impl WindowExtAndroid for Window {
  fn content_rect(&self) -> Rect {
    self.window.content_rect()
  }

  fn config(&self) -> Configuration {
    self.window.config()
  }

  fn activity_name(&self) -> String {
    self.window.activity_name().to_string()
  }

  fn create_embedded_activity(&self, activity_name: &str) -> Result<EmbeddedActivity, OsError> {
    self.window.create_embedded_activity(activity_name)
  }
}

impl<T> EventLoopWindowTargetExtAndroid for EventLoopWindowTarget<T> {}

/// Additional methods on `WindowBuilder` that are specific to Android.
pub trait WindowBuilderExtAndroid {
  /// The name of the activity class to create for this window.
  ///
  /// Each window is an activity in a task of its own, which the system shows as a separate window:
  /// it has its own entry in Recents, its own split-screen pane and its own window in desktop windowing.
  ///
  /// When not set, the window uses the activity the system started on its own, if no window uses it yet
  /// (the launcher activity on app start), and otherwise a new instance of the launcher activity.
  /// The activity must be declared in `AndroidManifest.xml` with a launch mode that allows an instance
  /// in each task, such as `standard` or `singleInstancePerTask`.
  fn with_activity_name(self, activity_name: String) -> Self;

  /// The name of the activity class that is opening this window.
  ///
  /// The new window's task is started from that activity.
  fn with_created_by_activity_name(self, created_by_activity_name: String) -> Self;
}

impl WindowBuilderExtAndroid for WindowBuilder {
  fn with_activity_name(mut self, activity_name: String) -> Self {
    self.platform_specific.activity_name = Some(activity_name);
    self
  }

  fn with_created_by_activity_name(mut self, created_by_activity_name: String) -> Self {
    self.platform_specific.created_by_activity_name = Some(created_by_activity_name);
    self
  }
}
