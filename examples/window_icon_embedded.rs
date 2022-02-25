// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// like window_icon.rs but the icon is embedded into the binary with `include_bytes!()`

use image::ImageFormat;

use tao::{
  event::Event,
  event_loop::{ControlFlow, EventLoop},
  window::{Icon, WindowBuilder},
};

#[allow(clippy::single_match)]
fn main() {
  env_logger::init();

  let icon = load_icon(&include_bytes!("./icon.png").to_vec());

  let event_loop = EventLoop::new();

  let window = WindowBuilder::new()
    .with_title("An iconic window!")
    // At present, this only does anything on Windows and Linux, so if you want to save load
    // time, you can put icon loading behind a function that returns `None` on other platforms.
    .with_window_icon(Some(icon))
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    if let Event::WindowEvent { event, .. } = event {
      use tao::event::WindowEvent::*;
      match event {
        CloseRequested => *control_flow = ControlFlow::Exit,
        _ => (),
      }
    }
  });
}

fn load_icon(bytes: &Vec<u8>) -> Icon {
  let (icon_rgba, icon_width, icon_height) = {
    let imagebuffer = image::load_from_memory_with_format(&bytes, ImageFormat::Png)
      .expect("Failed to open icon path")
      .into_rgba8();
    let (width, height) = imagebuffer.dimensions();
    let rgba = imagebuffer.into_raw();
    (rgba, width, height)
  };
  Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}
