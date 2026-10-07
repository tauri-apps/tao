// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

//! Regression check for closed windows leaking on macOS (tao#1327).
//!
//! Creates and closes `TAO_LEAK_CYCLES` windows (default 50) one after the
//! other, the way an app with pop-out windows does, then counts the instances
//! of tao's Objective-C classes that are still alive with `heap(1)`, which
//! walks the malloc zones of this process. Every count must be zero.
//!
//! Each window gets a plain `NSView` subclass as its content view, installed
//! the way wry installs its `WryWebViewParent`, so a view that only the window
//! keeps alive is counted too.
//!
//! The windows are never shown and the app is never activated, unless
//! `TAO_LEAK_VISIBLE=1` is set.
//!
//! ```sh
//! cargo run --example window_close_leak
//! ```

#[cfg(target_os = "macos")]
fn main() {
  macos::main();
}

#[cfg(not(target_os = "macos"))]
fn main() {
  println!("This example only runs on macOS.");
}

#[cfg(target_os = "macos")]
mod macos {
  use std::{
    process::Command,
    time::{Duration, Instant},
  };

  use objc2::{define_class, msg_send, rc::Retained, MainThreadOnly};
  use objc2_app_kit::{NSView, NSWindow};
  use objc2_foundation::MainThreadMarker;
  use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoop},
    platform::macos::{ActivationPolicy, EventLoopExtMacOS, WindowExtMacOS},
    window::{Window, WindowBuilder},
  };

  define_class!(
    // Stands in for wry's `WryWebViewParent`: a view owned only by the window
    // once the window has been closed.
    #[unsafe(super(NSView))]
    #[name = "TaoLeakCheckContentView"]
    struct ContentView;
  );

  impl ContentView {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
      unsafe { msg_send![Self::alloc(mtm), init] }
    }
  }

  /// The classes whose live instances must not outlive their window.
  const CLASSES: &[&str] = &[
    "TaoWindow",
    "NSKVONotifying_TaoWindow",
    "TaoWindowDelegate",
    "TaoView",
    "TaoLeakCheckContentView",
  ];

  pub fn main() {
    let cycles: usize = std::env::var("TAO_LEAK_CYCLES")
      .ok()
      .and_then(|value| value.parse().ok())
      .unwrap_or(50);
    let visible = std::env::var("TAO_LEAK_VISIBLE").is_ok_and(|value| value == "1");
    // Time each window lives, and time left for AppKit to finish closing the
    // last one before counting.
    let step = Duration::from_millis(if visible { 100 } else { 10 });
    let settle = Duration::from_secs(1);

    let mut event_loop = EventLoop::new();
    event_loop.set_activation_policy(if visible {
      ActivationPolicy::Accessory
    } else {
      ActivationPolicy::Prohibited
    });
    event_loop.set_activate_ignoring_other_apps(false);

    let mut window: Option<(Window, Retained<ContentView>)> = None;
    let mut closed = 0;
    let mut deadline = Instant::now();

    event_loop.run(move |event, event_loop, control_flow| match event {
      Event::NewEvents(StartCause::Init)
      | Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
        if let Some((window, content_view)) = window.take() {
          // Dropping the window closes it.
          drop(window);
          drop(content_view);
          closed += 1;
        } else if closed < cycles {
          let new_window = WindowBuilder::new()
            .with_title(format!("window_close_leak {}", closed + 1))
            .with_visible(visible)
            .with_focused(false)
            .build(event_loop)
            .unwrap();
          let content_view = ContentView::new(MainThreadMarker::new().unwrap());
          let ns_window = new_window.ns_window() as *const NSWindow;
          unsafe { (*ns_window).setContentView(Some(&content_view)) };
          window = Some((new_window, content_view));
        }

        if window.is_some() || closed < cycles {
          deadline = Instant::now() + step;
          *control_flow = ControlFlow::WaitUntil(deadline);
        } else if deadline.elapsed() < settle {
          *control_flow = ControlFlow::WaitUntil(deadline + settle);
        } else {
          *control_flow = ControlFlow::ExitWithCode(report(closed));
        }
      }
      _ => (),
    });
  }

  /// Prints the live instance count of each class in [`CLASSES`] and returns
  /// the exit code: 0 when none is alive.
  fn report(closed: usize) -> i32 {
    let output = Command::new("/usr/bin/heap")
      .arg(std::process::id().to_string())
      .output()
      .expect("failed to run heap(1)");
    let heap = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
      eprintln!("{}", String::from_utf8_lossy(&output.stderr));
      return 2;
    }

    println!("windows created and closed: {closed}");
    let mut leaked = 0;
    for class in CLASSES {
      let (count, bytes) = live_instances(&heap, class);
      leaked += count;
      println!("live {class}: {count} ({bytes} bytes)");
    }
    if leaked == 0 {
      println!("PASS: no instance outlived its window");
      0
    } else {
      println!("FAIL: {leaked} instances outlived their window");
      1
    }
  }

  /// Reads the COUNT and BYTES columns of `class`'s rows in `heap(1)`'s
  /// per-class table.
  fn live_instances(heap: &str, class: &str) -> (usize, usize) {
    heap
      .lines()
      .filter(|line| line.split_whitespace().any(|token| token == class))
      .filter_map(|line| {
        let mut columns = line.split_whitespace();
        let count = columns.next()?.parse::<usize>().ok()?;
        let bytes = columns.next()?.parse::<usize>().ok()?;
        Some((count, bytes))
      })
      .fold((0, 0), |(count, bytes), row| (count + row.0, bytes + row.1))
  }
}
