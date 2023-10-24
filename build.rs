// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

fn main() {
  // link carbon hotkey on macOS
  #[cfg(not(feature = "dox"))]
  {
    if std::env::var("CARGO_CFG_TARGET_OS").map_or(false, |os| os == "macos") {
      println!("cargo:rustc-link-lib=framework=Carbon");
      cc::Build::new()
        .file("src/platform_impl/macos/carbon_hotkey/carbon_hotkey_binding.c")
        .compile("carbon_hotkey_binding.a");
    }
  }
}
