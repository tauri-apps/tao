// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

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
//! - `global_shortcut` (available on `windows`, `unix`, `macos`)
//! - `run_return` (available on `windows`, `unix`, `macos`, and `android`)
//!
//! However only the module corresponding to the platform you're compiling to will be available.

pub mod android;
pub mod ios;
pub mod macos;
pub mod run_return;
pub mod unix;
pub mod windows;
