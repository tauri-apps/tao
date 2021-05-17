// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use simple_logger::SimpleLogger;
use tao::event_loop::EventLoop;

#[allow(clippy::single_match)]
fn main() {
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();
  let monitor = match event_loop.primary_monitor() {
    Some(monitor) => monitor,
    None => {
      println!("No primary monitor detected.");
      return;
    }
  };

  println!("Listing available video modes:");

  for mode in monitor.video_modes() {
    println!("{}", mode);
  }
}
