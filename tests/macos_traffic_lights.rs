// Copyright 2026 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// AppKit requires the process main thread, so this test uses harness = false.
// Run with -- --fullscreen to also exercise a native fullscreen transition.
#[cfg(all(target_os = "macos", not(miri)))]
fn main() {
  native::run();
}

#[cfg(not(all(target_os = "macos", not(miri))))]
fn main() {}

#[cfg(all(target_os = "macos", not(miri)))]
mod native {
  use std::{
    cell::Cell,
    time::{Duration, Instant},
  };

  use objc2::{define_class, msg_send, rc::Retained, DefinedClass, MainThreadOnly};
  use objc2_app_kit::{
    NSView, NSViewFrameDidChangeNotification, NSWindow, NSWindowButton, NSWindowStyleMask,
  };
  use objc2_foundation::{
    MainThreadMarker, NSDate, NSNotification, NSNotificationCenter, NSObject, NSRect, NSRunLoop,
    NSString,
  };
  use tao::{
    dpi::{LogicalPosition, PhysicalPosition},
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoop},
    platform::{
      macos::{WindowBuilderExtMacOS, WindowExtMacOS},
      run_return::EventLoopExtRunReturn,
    },
    window::{Fullscreen, WindowBuilder},
  };

  define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Cell<usize>]
    struct FrameCounter;

    impl FrameCounter {
      #[unsafe(method(frameChanged:))]
      fn frame_changed(&self, _notification: &NSNotification) {
        self.ivars().set(self.ivars().get() + 1);
      }
    }
  );

  fn pump() {
    NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.01));
  }

  fn wait_for(description: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
      assert!(Instant::now() < deadline, "timed out: {description}");
      pump();
    }
  }

  fn frames(window: &NSWindow) -> [NSRect; 4] {
    unsafe {
      let buttons = [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
      ]
      .map(|kind| window.standardWindowButton(kind).unwrap());
      [
        NSView::frame(&buttons[0]),
        NSView::frame(&buttons[1]),
        NSView::frame(&buttons[2]),
        buttons[0].superview().unwrap().superview().unwrap().frame(),
      ]
    }
  }

  fn has_inset(window: &NSWindow, x: f64, y: f64) -> bool {
    let [close, miniaturize, zoom, container] = frames(window);
    let spacing = miniaturize.origin.x - close.origin.x;
    close.origin.x == x
      && spacing > 0.0
      && zoom.origin.x == x + spacing * 2.0
      && container.size.height == close.size.height + y
      && container.origin.y == window.frame().size.height - container.size.height
  }

  #[allow(deprecated)]
  pub fn run() {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
      .with_title("Traffic light regression")
      .with_titlebar_transparent(true)
      .with_title_hidden(true)
      .with_fullsize_content_view(true)
      .with_traffic_light_inset(LogicalPosition::new(19.0, 24.0))
      .build(&event_loop)
      .unwrap();
    let parent = unsafe { &*window.ns_view().cast::<NSView>() };
    let ns_window = parent.window().unwrap();
    let parent = ns_window.contentView().unwrap();
    unsafe { ns_window.setAutodisplay(false) };
    wait_for("initial attachment", || has_inset(&ns_window, 19.0, 24.0));
    for _ in 0..5 {
      pump();
    }

    ns_window.setTitle(&NSString::from_str("Native title change without drawing"));
    wait_for("native title", || has_inset(&ns_window, 19.0, 24.0));

    ns_window.setTitle(&NSString::from_str("Queue repair of the old inset"));
    window.set_traffic_light_inset(LogicalPosition::new(30.0, 34.0));
    assert!(has_inset(&ns_window, 30.0, 34.0), "setter is immediate");
    ns_window.setTitle(&NSString::from_str("Use the latest runtime inset"));
    wait_for("runtime inset", || has_inset(&ns_window, 30.0, 34.0));

    for i in 0..10 {
      ns_window.setTitle(&NSString::from_str(&format!("Repeated native layout {i}")));
      unsafe {
        parent.setNeedsLayout(true);
        parent.layoutSubtreeIfNeeded();
      }
    }
    wait_for("coalesced layout", || has_inset(&ns_window, 30.0, 34.0));
    for _ in 0..5 {
      unsafe {
        parent.setNeedsDisplay(true);
        parent.displayIfNeeded();
      }
      pump();
      assert!(has_inset(&ns_window, 30.0, 34.0));
    }

    let mut rect = ns_window.frame();
    rect.size.width += 100.0;
    rect.size.height += 80.0;
    ns_window.setFrame_display(rect, false);
    wait_for("resize", || has_inset(&ns_window, 30.0, 34.0));

    let replacement = unsafe { NSView::new(mtm) };
    ns_window.setTitle(&NSString::from_str("Queue repair before detaching"));
    ns_window.setContentView(Some(&replacement));
    ns_window.setTitle(&NSString::from_str("Detached content"));
    let detached = frames(&ns_window);
    let scale = ns_window.backingScaleFactor();
    window.set_traffic_light_inset(PhysicalPosition::new(24.0 * scale, 26.0 * scale));
    for _ in 0..5 {
      pump();
    }
    assert_eq!(
      frames(&ns_window),
      detached,
      "detached views must not repair"
    );
    ns_window.setContentView(Some(&parent));
    wait_for("reattachment uses current inset", || {
      has_inset(&ns_window, 24.0, 26.0)
    });
    window.set_traffic_light_inset(LogicalPosition::new(30.0, 34.0));

    unsafe {
      let close = ns_window
        .standardWindowButton(NSWindowButton::CloseButton)
        .unwrap();
      close.setHidden(true);
      ns_window.setTitle(&NSString::from_str("Hidden controls"));
      wait_for("hidden controls", || has_inset(&ns_window, 30.0, 34.0));
      assert!(close.isHidden(), "repair must preserve visibility");
      close.setHidden(false);
      parent.addSubview(&replacement);

      for kind in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
      ] {
        let button = ns_window.standardWindowButton(kind).unwrap();
        let buttons_view = button.superview().unwrap();
        button.removeFromSuperview();
        replacement.addSubview(&button);
        let mut origin = NSView::frame(&button).origin;
        origin.x += 123.0;
        button.setFrameOrigin(origin);
        let before = NSView::frame(&button);
        parent.setNeedsDisplay(true);
        parent.displayIfNeeded();
        for _ in 0..5 {
          pump();
        }
        assert_eq!(
          NSView::frame(&button),
          before,
          "foreign controls are untouched"
        );
        button.removeFromSuperview();
        buttons_view.addSubview(&button);
        ns_window.setTitle(&NSString::from_str(&format!("Reattached {kind:?}")));
        wait_for(&format!("reattached {kind:?}"), || {
          has_inset(&ns_window, 30.0, 34.0)
        });
      }
      replacement.removeFromSuperview();
    }

    let other = WindowBuilder::new()
      .with_title("No inset")
      .with_titlebar_transparent(true)
      .with_fullsize_content_view(true)
      .build(&event_loop)
      .unwrap();
    let other_native = unsafe { &*other.ns_window().cast::<NSWindow>() };
    unsafe { other_native.setAutodisplay(false) };
    other_native.setTitle(&NSString::from_str("Native layout without an inset"));
    let other_frames = frames(other_native);
    for _ in 0..5 {
      pump();
    }
    assert_eq!(frames(other_native), other_frames);
    assert!(
      has_inset(&ns_window, 30.0, 34.0),
      "observers are window-scoped"
    );
    drop(other);

    if std::env::args().any(|arg| arg == "--fullscreen") {
      unsafe { ns_window.setAutodisplay(true) };
      let deadline = Instant::now() + Duration::from_secs(15);
      let mut entered = None;
      let mut leaving = false;
      let mut recovered = false;
      event_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(10));
        if matches!(event, Event::NewEvents(StartCause::Init)) {
          window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        let fullscreen = ns_window
          .styleMask()
          .contains(NSWindowStyleMask::FullScreen);
        if fullscreen && entered.is_none() {
          entered = Some(Instant::now());
        }
        if !leaving && entered.is_some_and(|time: Instant| time.elapsed() > Duration::from_secs(2))
        {
          leaving = true;
          window.set_fullscreen(None);
        }
        if leaving && !fullscreen && has_inset(&ns_window, 30.0, 34.0) {
          recovered = true;
          *control_flow = ControlFlow::Exit;
        }
        if Instant::now() > deadline {
          window.set_fullscreen(None);
          *control_flow = ControlFlow::Exit;
        }
      });
      assert!(recovered, "fullscreen transition must recover the inset");
      unsafe { ns_window.setAutodisplay(false) };
    }

    for _ in 0..5 {
      pump();
    }
    let counter = FrameCounter::alloc(mtm).set_ivars(Cell::new(0));
    let counter: Retained<FrameCounter> = unsafe { msg_send![super(counter), init] };
    let center = NSNotificationCenter::defaultCenter();
    unsafe {
      let close = ns_window
        .standardWindowButton(NSWindowButton::CloseButton)
        .unwrap();
      let miniaturize = ns_window
        .standardWindowButton(NSWindowButton::MiniaturizeButton)
        .unwrap();
      let zoom = ns_window
        .standardWindowButton(NSWindowButton::ZoomButton)
        .unwrap();
      let container = close.superview().unwrap().superview().unwrap();
      for view in [
        &*container,
        &*close as &NSView,
        &*miniaturize as &NSView,
        &*zoom as &NSView,
      ] {
        center.addObserver_selector_name_object(
          &counter,
          objc2::sel!(frameChanged:),
          Some(NSViewFrameDidChangeNotification),
          Some(view),
        );
      }
    }
    let stable = frames(&ns_window);
    for _ in 0..10 {
      pump();
    }
    assert_eq!(frames(&ns_window), stable);
    assert_eq!(counter.ivars().get(), 0, "recovery must become idle");
    unsafe { center.removeObserver(&counter) };

    ns_window.setTitle(&NSString::from_str("Queue repair before teardown"));
    drop(window);
    let after_drop = frames(&ns_window);
    for _ in 0..5 {
      pump();
    }
    assert_eq!(
      frames(&ns_window),
      after_drop,
      "drop cancels pending repairs"
    );
    ns_window.setTitle(&NSString::from_str("After teardown"));
    let after_title = frames(&ns_window);
    for _ in 0..5 {
      pump();
    }
    assert_eq!(frames(&ns_window), after_title, "drop removes observers");
    drop(parent);
    drop(ns_window);
    for _ in 0..5 {
      pump();
    }
    eprintln!("macOS traffic-light native regression passed");
  }
}
