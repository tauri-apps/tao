// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Contains traits with platform-specific methods in them.
//!
//! Contains the follow OS-specific modules:
//!
//!  - `android`
//!  - `ios`
//!  - `macos`
//!  - `unix`
//!  - `windows`
//!
//! And the following platform-specific module:
//!
//! - `run_return` (available on `windows`, `unix`, `macos`, and `android`)
//! - `status_bar` (available on `windows`, `unix`, and `macos`)
//!
//! However only the module corresponding to the platform you're compiling to will be available.

pub mod android;
pub mod ios;
pub mod macos;
pub mod unix;
pub mod windows;

pub mod run_return;
pub mod system_tray;
