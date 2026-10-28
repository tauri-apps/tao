// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{platform::windows::WindowExtWindows, window::WindowBuilder};
use std::cell::RefCell;

#[derive(Debug, PartialEq, Eq)]
enum PaintEvent {
  MainEventsCleared,
  Redraw(RootWindowId),
  RedrawEventsCleared,
}

struct Fixture {
  event_loop: EventLoop<()>,
  events: Rc<RefCell<Vec<PaintEvent>>>,
  first: crate::window::Window,
  second: crate::window::Window,
}

impl Fixture {
  unsafe fn new() -> Self {
    let event_loop = EventLoop::<()>::new(&mut PlatformSpecificEventLoopAttributes {
      any_thread: true,
      dpi_aware: false,
      ..Default::default()
    });
    let first = WindowBuilder::new()
      .with_title("Tao paint regression A")
      .build(event_loop.window_target())
      .unwrap();
    let second = WindowBuilder::new()
      .with_title("Tao paint regression B")
      .build(event_loop.window_target())
      .unwrap();
    let events = Rc::new(RefCell::new(Vec::new()));
    let received = events.clone();
    let runner = &event_loop.window_target.p.runner_shared;
    runner.set_event_handler(move |event, flow| {
      *flow = ControlFlow::Wait;
      let event = match event {
        Event::MainEventsCleared => PaintEvent::MainEventsCleared,
        Event::RedrawRequested(id) => PaintEvent::Redraw(id),
        Event::RedrawEventsCleared => PaintEvent::RedrawEventsCleared,
        _ => return,
      };
      received.borrow_mut().push(event);
    });
    runner.poll();
    assert!(runner.take_panic_error().is_ok());
    events.borrow_mut().clear();
    Self {
      event_loop,
      events,
      first,
      second,
    }
  }

  fn runner(&self) -> &EventLoopRunnerShared<()> {
    &self.event_loop.window_target.p.runner_shared
  }

  fn hwnd(window: &crate::window::Window) -> HWND {
    HWND(window.hwnd() as _)
  }
}

/// A native dispatch of the thread event target's `WM_PAINT` can land while a public window's
/// `WM_PAINT` handler owns the redraw flush (`redrawing`) and no user callback is running
/// (`!should_buffer`). The state is entered deterministically here because the OS paint
/// scheduling that would deliver it natively is not reproducible in a test.
#[test]
fn nested_target_paint_preserves_redraw_batch() {
  unsafe {
    let fixture = Fixture::new();
    let runner = fixture.runner();
    let target = runner.thread_msg_target();
    let (first, second) = (
      Fixture::hwnd(&fixture.first),
      Fixture::hwnd(&fixture.second),
    );
    assert_ne!(target, first);
    assert_ne!(target, second);
    assert_ne!(first, second);

    // The outer owner starts the flush, exactly as `flush_paint_messages` does.
    assert!(flush_paint_messages(Some(first), runner));
    assert_eq!(
      (
        runner.redrawing(),
        runner.handling_events(),
        runner.should_buffer()
      ),
      (true, true, false)
    );

    // Nested native dispatch of the target's WM_PAINT between callbacks.
    SendMessageW(target, WM_PAINT, None, None);
    assert!(
      runner.take_panic_error().is_ok(),
      "nested target WM_PAINT panicked during the outer flush"
    );
    assert!(
      runner.redrawing(),
      "nested paint must not complete the flush"
    );
    assert_eq!(*fixture.events.borrow(), [PaintEvent::MainEventsCleared]);

    // Remaining redraws are delivered in order and the owner completes exactly once.
    SendMessageW(second, WM_PAINT, None, None);
    SendMessageW(first, WM_PAINT, None, None);
    runner.redraw_events_cleared();
    assert_eq!(
      *fixture.events.borrow(),
      [
        PaintEvent::MainEventsCleared,
        PaintEvent::Redraw(fixture.second.id()),
        PaintEvent::Redraw(fixture.first.id()),
        PaintEvent::RedrawEventsCleared,
      ]
    );
    assert!(runner.take_panic_error().is_ok());
    assert!(!runner.handling_events());
    assert!(!runner.should_buffer());
    assert_eq!(runner.control_flow(), ControlFlow::Wait);
    runner.reset_runner();
  }
}

/// Control: while the user callback is running, the target's WM_PAINT is buffered.
#[test]
fn target_paint_during_user_callback_is_buffered() {
  unsafe {
    let event_loop = EventLoop::<()>::new(&mut PlatformSpecificEventLoopAttributes {
      any_thread: true,
      dpi_aware: false,
      ..Default::default()
    });
    let runner = &event_loop.window_target.p.runner_shared;
    let target = runner.thread_msg_target();
    let observed = Rc::new(Cell::new(None));
    let seen = observed.clone();
    let events = Rc::new(RefCell::new(Vec::new()));
    let received = events.clone();
    let callback_runner = runner.clone();
    runner.set_event_handler(move |event, flow| {
      *flow = ControlFlow::Wait;
      match event {
        Event::NewEvents(_) if seen.get().is_none() => {
          seen.set(Some((
            callback_runner.redrawing(),
            callback_runner.should_buffer(),
          )));
          SendMessageW(target, WM_PAINT, None, None);
        }
        Event::RedrawEventsCleared => received.borrow_mut().push(PaintEvent::RedrawEventsCleared),
        _ => {}
      }
    });
    runner.poll();
    assert!(runner.take_panic_error().is_ok());
    assert_eq!(observed.get(), Some((false, true)));
    assert!(events.borrow().is_empty(), "buffered paint must not flush");
    assert!(runner.handling_events());
    runner.reset_runner();
  }
}
