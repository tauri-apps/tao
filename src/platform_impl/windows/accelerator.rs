// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

use lazy_static::lazy_static;

use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};

// NOTE:
// https://docs.microsoft.com/en-us/windows/win32/wsw/thread-safety
// All handles you obtain from functions in Kernel32 are thread-safe,
// unless the MSDN Library article for the function explicitly mentions it is not.

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct WindowHandle(isize);
unsafe impl Send for WindowHandle {}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct AccelHandle(isize);
unsafe impl Send for AccelHandle {}
unsafe impl Sync for AccelHandle {}

lazy_static! {
  static ref ACCEL_TABLES: Mutex<HashMap<WindowHandle, Arc<AccelTable>>> =
    Mutex::new(HashMap::default());
}

/// A Accelerators Table for Windows
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct AccelTable {
  accel: AccelHandle,
}

impl AccelTable {
  fn new(accel: &[ACCEL]) -> AccelTable {
    let accel = unsafe { CreateAcceleratorTableW(accel as *const _ as *mut _, accel.len() as i32) };
    AccelTable {
      accel: AccelHandle(accel.0),
    }
  }

  pub(crate) fn handle(&self) -> HACCEL {
    HACCEL(self.accel.0)
  }
}

pub(crate) fn register_accel(hwnd: HWND, accel: &[ACCEL]) {
  let mut table = ACCEL_TABLES.lock().unwrap();
  table.insert(WindowHandle(hwnd.0), Arc::new(AccelTable::new(accel)));
}

impl Drop for AccelTable {
  fn drop(&mut self) {
    unsafe {
      DestroyAcceleratorTable(self.handle());
    }
  }
}

pub(crate) fn find_accels(hwnd: HWND) -> Option<Arc<AccelTable>> {
  let table = ACCEL_TABLES.lock().unwrap();
  table.get(&WindowHandle(hwnd.0)).cloned()
}
