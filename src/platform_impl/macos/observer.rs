// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  self,
  ffi::c_void,
  panic::{AssertUnwindSafe, UnwindSafe},
  ptr,
  rc::Weak,
  time::Instant,
};

use objc2_core_foundation::{
  kCFRunLoopCommonModes, CFAbsoluteTimeGetCurrent, CFIndex, CFRetained, CFRunLoop,
  CFRunLoopActivity, CFRunLoopObserver, CFRunLoopObserverCallBack, CFRunLoopObserverContext,
  CFRunLoopTimer,
};

use crate::platform_impl::platform::{
  app_state::AppState,
  event_loop::{stop_app_on_panic, PanicInfo},
};

unsafe fn control_flow_handler<F>(panic_info: *mut c_void, f: F)
where
  F: FnOnce(Weak<PanicInfo>) + UnwindSafe,
{
  let info_from_raw = Weak::from_raw(panic_info as *mut PanicInfo);
  // Asserting unwind safety on this type should be fine because `PanicInfo` is
  // `RefUnwindSafe` and `Rc<T>` is `UnwindSafe` if `T` is `RefUnwindSafe`.
  let panic_info = AssertUnwindSafe(Weak::clone(&info_from_raw));
  // `from_raw` takes ownership of the data behind the pointer.
  // But if this scope takes ownership of the weak pointer, then
  // the weak pointer will get free'd at the end of the scope.
  // However we want to keep that weak reference around after the function.
  std::mem::forget(info_from_raw);

  stop_app_on_panic(Weak::clone(&panic_info), move || {
    let _ = &panic_info;
    f(panic_info.0)
  });
}

// begin is queued with the highest priority to ensure it is processed before other observers
extern "C-unwind" fn control_flow_begin_handler(
  _: *mut CFRunLoopObserver,
  activity: CFRunLoopActivity,
  panic_info: *mut c_void,
) {
  unsafe {
    control_flow_handler(panic_info, |panic_info| {
      match activity {
        CFRunLoopActivity::AfterWaiting => {
          //trace!("Triggered `CFRunLoopAfterWaiting`");
          AppState::wakeup(panic_info);
          //trace!("Completed `CFRunLoopAfterWaiting`");
        }
        _ => unreachable!(),
      }
    });
  }
}

// end is queued with the lowest priority to ensure it is processed after other observers
// without that, LoopDestroyed would  get sent after MainEventsCleared
extern "C-unwind" fn control_flow_end_handler(
  _: *mut CFRunLoopObserver,
  activity: CFRunLoopActivity,
  panic_info: *mut c_void,
) {
  unsafe {
    control_flow_handler(panic_info, |panic_info| {
      match activity {
        CFRunLoopActivity::BeforeWaiting => {
          //trace!("Triggered `CFRunLoopBeforeWaiting`");
          AppState::cleared(panic_info);
          //trace!("Completed `CFRunLoopBeforeWaiting`");
        }
        CFRunLoopActivity::Exit => (), //unimplemented!(), // not expected to ever happen
        _ => unreachable!(),
      }
    });
  }
}

struct RunLoop(CFRetained<CFRunLoop>);

impl RunLoop {
  fn get() -> Self {
    RunLoop(CFRunLoop::main().unwrap())
  }

  unsafe fn add_observer(
    &self,
    flags: CFRunLoopActivity,
    priority: CFIndex,
    handler: CFRunLoopObserverCallBack,
    context: *mut CFRunLoopObserverContext,
  ) {
    let observer = CFRunLoopObserver::new(None, flags.0, true, priority, handler, context).unwrap();
    self.0.add_observer(Some(&observer), kCFRunLoopCommonModes);
  }
}

pub fn setup_control_flow_observers(panic_info: Weak<PanicInfo>) {
  unsafe {
    let mut context = CFRunLoopObserverContext {
      info: Weak::into_raw(panic_info) as *mut _,
      version: 0,
      retain: None,
      release: None,
      copyDescription: None,
    };
    let run_loop = RunLoop::get();
    run_loop.add_observer(
      CFRunLoopActivity::AfterWaiting,
      CFIndex::MIN,
      Some(control_flow_begin_handler),
      &mut context as *mut _,
    );
    run_loop.add_observer(
      CFRunLoopActivity::Exit | CFRunLoopActivity::BeforeWaiting,
      CFIndex::MAX,
      Some(control_flow_end_handler),
      &mut context as *mut _,
    );
  }
}

pub struct EventLoopWaker {
  timer: CFRetained<CFRunLoopTimer>,
}

impl Drop for EventLoopWaker {
  fn drop(&mut self) {
    self.timer.invalidate();
  }
}

impl Default for EventLoopWaker {
  fn default() -> EventLoopWaker {
    extern "C-unwind" fn wakeup_main_loop(_timer: *mut CFRunLoopTimer, _info: *mut c_void) {}
    unsafe {
      // Create a timer with a 0.1µs interval (1ns does not work) to mimic polling.
      // It is initially setup with a first fire time really far into the
      // future, but that gets changed to fire immediately in did_finish_launching
      let timer = CFRunLoopTimer::new(
        None,
        f64::MAX,
        0.000_000_1,
        0,
        0,
        Some(wakeup_main_loop),
        ptr::null_mut(),
      )
      .unwrap();
      CFRunLoop::main()
        .unwrap()
        .add_timer(Some(&timer), kCFRunLoopCommonModes);
      EventLoopWaker { timer }
    }
  }
}

impl EventLoopWaker {
  pub fn stop(&mut self) {
    self.timer.set_next_fire_date(f64::MAX);
  }

  pub fn start(&mut self) {
    self.timer.set_next_fire_date(f64::MIN);
  }

  pub fn start_at(&mut self, instant: Instant) {
    let now = Instant::now();
    if now >= instant {
      self.start();
    } else {
      let current = CFAbsoluteTimeGetCurrent();
      let duration = instant - now;
      let fsecs = duration.subsec_nanos() as f64 / 1_000_000_000.0 + duration.as_secs() as f64;
      self.timer.set_next_fire_date(current + fsecs);
    }
  }
}
