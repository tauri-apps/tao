// Copyright 2021-2026 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

//! Exercise continuous GTK drawing while Tao waits, without Tauri or WebKit.
//! Run `cargo run --example linux_redraw_queue -- 15` on Linux.
//! Add `wait-until` after the duration to exercise `ControlFlow::WaitUntil`.
//! Run without mouse/keyboard input; the example exits automatically.
//! The RESULT is captured before sending the final user event to stop the loop.

#[cfg(target_os = "linux")]
fn main() {
  linux::run();
}

#[cfg(not(target_os = "linux"))]
fn main() {
  eprintln!("This example requires Linux and GTK.");
}

#[cfg(target_os = "linux")]
mod linux {
  use gtk::{glib, prelude::*};
  use std::{
    cell::Cell,
    fs,
    rc::Rc,
    time::{Duration, Instant},
  };
  use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    platform::unix::WindowExtUnix,
    window::WindowBuilder,
  };

  #[derive(Clone, Copy, Debug)]
  enum UserEvent {
    Finish(i32),
  }

  #[derive(Default)]
  struct Counts {
    gtk: Cell<u64>,
    tao: Cell<u64>,
    ticks: Cell<u64>,
    baseline: Cell<Option<(u64, u64, u64)>>,
    last_second: Cell<u64>,
  }

  fn rss_kib() -> u64 {
    fs::read_to_string("/proc/self/status")
      .ok()
      .and_then(|status| {
        status.lines().find_map(|line| {
          line
            .strip_prefix("VmRSS:")?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
        })
      })
      .unwrap_or(0)
  }

  pub fn run() {
    let seconds: u64 = std::env::args()
      .nth(1)
      .unwrap_or_else(|| "15".into())
      .parse()
      .expect("usage: tao-redraw-repro [measurement-seconds] [wait-until]");
    assert!((3..=3600).contains(&seconds));
    let wait_until = std::env::args().nth(2).as_deref() == Some("wait-until");
    let requested_flow = if wait_until {
      ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(7200))
    } else {
      ControlFlow::Wait
    };
    let warmup = Duration::from_secs(2);
    let finish_at = warmup + Duration::from_secs(seconds);
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let window = WindowBuilder::new()
      .with_title("Tao GTK draw queue reproducer")
      .with_inner_size(tao::dpi::LogicalSize::new(320., 200.))
      .build(&event_loop)
      .expect("create GTK window");
    let gtk_window = window.gtk_window().clone();
    let counts = Rc::new(Counts::default());
    let draw_counts = counts.clone();
    gtk_window.connect_draw(move |_, _| {
      draw_counts.gtk.set(draw_counts.gtk.get() + 1);
      glib::Propagation::Proceed
    });

    let proxy = event_loop.create_proxy();
    let timer_counts = counts.clone();
    let started = Instant::now();
    println!(
      "CONFIG warmup_s=2 measure_s={seconds} gtk_timer_ms=16 control_flow={}",
      if wait_until { "WaitUntil" } else { "Wait" }
    );
    // This GTK source wakes the GLib loop, but sends no normal Tao event while
    // measuring. It exercises the GTK connect_draw producer, not request_redraw.
    glib::timeout_add_local(Duration::from_millis(16), move || {
      let elapsed = started.elapsed();
      if elapsed >= warmup && timer_counts.baseline.get().is_none() {
        timer_counts.baseline.set(Some((
          timer_counts.gtk.get(),
          timer_counts.tao.get(),
          rss_kib(),
        )));
        println!("MEASUREMENT_START elapsed_s={:.3}", elapsed.as_secs_f64());
      }
      if let Some((base_gtk, base_tao, base_rss)) = timer_counts.baseline.get() {
        let gtk_draws = timer_counts.gtk.get() - base_gtk;
        let tao_redraws = timer_counts.tao.get() - base_tao;
        let second = elapsed.as_secs();
        if second != timer_counts.last_second.replace(second) {
          println!(
            "SAMPLE elapsed_s={:.3} gtk_draws={gtk_draws} tao_redraws={tao_redraws} rss_kib={}",
            elapsed.as_secs_f64(),
            rss_kib()
          );
        }
        if elapsed >= finish_at {
          let enough_draws = gtk_draws >= seconds * 20;
          let delivered = tao_redraws * 100 >= gtk_draws * 90;
          let pass = enough_draws && delivered;
          // Snapshot before sending the only user event: shutdown itself
          // can wake Tao and must not count as successful redraw delivery.
          println!("RESULT pass={pass} gtk_draws={gtk_draws} tao_redraws={tao_redraws} timer_ticks={} rss_start_kib={base_rss} rss_end_kib={} enough_draws={enough_draws} delivered={delivered}", timer_counts.ticks.get(), rss_kib());
          proxy
            .send_event(UserEvent::Finish(if pass { 0 } else { 2 }))
            .expect("send automatic shutdown event");
          return glib::ControlFlow::Break;
        }
      }
      timer_counts.ticks.set(timer_counts.ticks.get() + 1);
      gtk_window.queue_draw();
      glib::ControlFlow::Continue
    });

    event_loop.run(move |event, _, control_flow| {
      if !matches!(*control_flow, ControlFlow::ExitWithCode(_)) {
        *control_flow = requested_flow;
      }
      match event {
        Event::RedrawRequested(_) => counts.tao.set(counts.tao.get() + 1),
        Event::UserEvent(UserEvent::Finish(code)) => {
          *control_flow = ControlFlow::ExitWithCode(code)
        }
        Event::WindowEvent {
          window_id,
          event: WindowEvent::CloseRequested,
          ..
        } if window_id == window.id() => *control_flow = ControlFlow::ExitWithCode(3),
        _ => {}
      }
    });
  }
}
