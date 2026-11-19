// Copyright 2026 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(target_os = "android")]

use std::{
  io::Write,
  process::{Command, Stdio},
  thread,
  time::{Duration, Instant},
};
use tao::{
  event::Event,
  event_loop::{ControlFlow, EventLoop},
};

extern "C" fn exit_handler() {
  // SAFETY: `_exit` accepts any exit status and invokes no callbacks into Rust.
  unsafe { libc::_exit(99) }
}

#[test]
fn exits_without_running_libc_handlers() {
  const CHILD_CODE: &str = "TAO_ANDROID_EXIT_CHILD_CODE";

  if let Ok(code) = std::env::var(CHILD_CODE) {
    let code = code.parse().unwrap();
    ndk::looper::ThreadLooper::prepare();
    // SAFETY: `exit_handler` has the C ABI and remains valid for the process lifetime.
    assert_eq!(unsafe { libc::atexit(exit_handler) }, 0);
    EventLoop::new().run(move |event, _, control_flow| match event {
      Event::NewEvents(_) => *control_flow = ControlFlow::ExitWithCode(code),
      Event::LoopDestroyed => std::io::stdout().write_all(b"loop-destroyed\n").unwrap(),
      _ => {}
    });
  }

  for code in [0, 42] {
    let mut child = Command::new(std::env::current_exe().unwrap())
      .args([
        "--exact",
        "exits_without_running_libc_handlers",
        "--nocapture",
      ])
      .env(CHILD_CODE, code.to_string())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
      if child.try_wait().unwrap().is_some() {
        break;
      }
      if Instant::now() >= deadline {
        child.kill().unwrap();
        child.wait().unwrap();
        panic!("event loop did not exit with code {code}");
      }
      thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(code), "{output:?}");
    assert!(String::from_utf8(output.stdout)
      .unwrap()
      .contains("loop-destroyed\n"));
  }
}
