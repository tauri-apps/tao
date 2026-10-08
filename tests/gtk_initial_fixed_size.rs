// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]

use std::time::{Duration, Instant};

use gtk::prelude::*;
use tao::{
  dpi::LogicalSize,
  event::Event,
  event_loop::{ControlFlow, EventLoopBuilder},
  platform::{
    run_return::EventLoopExtRunReturn,
    unix::{EventLoopBuilderExtUnix, WindowExtUnix},
  },
  window::WindowBuilder,
};

#[test]
#[ignore = "requires a GTK display; run on a tiling Wayland compositor"]
fn fixed_size_is_applied_before_the_first_configure() {
  let mut event_loop = EventLoopBuilder::<()>::new().with_any_thread(true).build();
  let expected = LogicalSize::new(280, 404);
  let window = WindowBuilder::new()
    .with_title("Tao initial fixed-size regression")
    .with_visible(false)
    .with_decorations(false)
    .with_inner_size(expected)
    .with_resizable(false)
    .build(&event_loop)
    .unwrap();

  // The native flag must already be set while the window is still unmapped.
  assert!(!window.gtk_window().is_resizable());
  window.set_visible(true);

  let deadline = Instant::now() + Duration::from_secs(1);
  event_loop.run_return(|event, _, control_flow| {
    *control_flow = ControlFlow::WaitUntil(deadline);
    if matches!(event, Event::MainEventsCleared) && Instant::now() >= deadline {
      *control_flow = ControlFlow::Exit;
    }
  });

  assert_eq!(
    window.inner_size().to_logical::<u32>(window.scale_factor()),
    expected
  );
}
