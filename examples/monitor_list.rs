// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use simple_logger::SimpleLogger;
use tao::{event_loop::EventLoop, window::WindowBuilder};

fn main() {
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new().build(&event_loop).unwrap();

  dbg!(window.available_monitors().collect::<Vec<_>>());
  dbg!(window.primary_monitor());
}
