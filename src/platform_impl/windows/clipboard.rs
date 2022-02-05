// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::clipboard::{ClipboardFormat, FormatId};
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, ptr};
use windows::Win32::{
  Foundation::{HANDLE, HWND, PSTR, PWSTR},
  System::{
    DataExchange::{
      CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatA,
      SetClipboardData,
    },
    Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
    SystemServices::CF_UNICODETEXT,
  },
};

#[derive(Debug, Clone, Default)]
pub struct Clipboard;

impl Clipboard {
  pub fn write_text(&mut self, s: impl AsRef<str>) {
    let s = s.as_ref();
    let format: ClipboardFormat = s.into();
    self.put_formats(&[format])
  }

  pub(crate) fn read_text(&self) -> Option<String> {
    with_clipboard(|| unsafe {
      let handle = GetClipboardData(CF_UNICODETEXT);
      if handle.0 == 0 {
        None
      } else {
        let unic_str = PWSTR(GlobalLock(handle.0) as *mut _);
        let mut len = 0;
        while *unic_str.0.offset(len) != 0 {
          len += 1;
        }
        let utf16_slice = std::slice::from_raw_parts(unic_str.0, len as usize);
        let result = String::from_utf16(utf16_slice);
        if let Ok(result) = result {
          GlobalUnlock(handle.0);
          return Some(result);
        }

        None
      }
    })
    .flatten()
  }

  pub(crate) fn put_formats(&mut self, formats: &[ClipboardFormat]) {
    with_clipboard(|| unsafe {
      EmptyClipboard();

      for format in formats {
        let handle = make_handle(format);
        let format_id = match get_format_id(format.identifier) {
          Some(id) => id,
          None => {
            #[cfg(debug_assertions)]
            println!("failed to register clipboard format {}", &format.identifier);
            continue;
          }
        };
        let result = SetClipboardData(format_id, handle);
        if result.0 == 0 {
          #[cfg(debug_assertions)]
          println!(
            "failed to set clipboard for fmt {}, error: {}",
            &format.identifier,
            windows::core::Error::from_win32().code().0
          );
        }
      }
    });
  }
}

fn get_format_id(format: FormatId) -> Option<u32> {
  if let Some((id, _)) = STANDARD_FORMATS.iter().find(|(_, s)| s == &format) {
    return Some(*id);
  }
  match format {
    ClipboardFormat::TEXT => Some(CF_UNICODETEXT),
    other => register_identifier(other),
  }
}

fn register_identifier(ident: &str) -> Option<u32> {
  unsafe {
    let pb_format = RegisterClipboardFormatA(ident);
    if pb_format == 0 {
      #[cfg(debug_assertions)]
      println!(
        "failed to register clipboard format '{}'; error {}.",
        ident,
        windows::core::Error::from_win32().code().0
      );
      return None;
    }
    Some(pb_format)
  }
}

unsafe fn make_handle(format: &ClipboardFormat) -> HANDLE {
  HANDLE(if format.identifier == ClipboardFormat::TEXT {
    let s: &OsStr = std::str::from_utf8_unchecked(&format.data).as_ref();
    let wstr: Vec<u16> = s.encode_wide().chain(Some(0)).collect();
    let handle = GlobalAlloc(GMEM_MOVEABLE, wstr.len() * std::mem::size_of::<u16>());
    let locked = PWSTR(GlobalLock(handle) as *mut _);
    ptr::copy_nonoverlapping(wstr.as_ptr(), locked.0, wstr.len());
    GlobalUnlock(handle);
    handle
  } else {
    let handle = GlobalAlloc(GMEM_MOVEABLE, format.data.len() * std::mem::size_of::<u8>());
    let locked = PSTR(GlobalLock(handle) as *mut _);
    ptr::copy_nonoverlapping(format.data.as_ptr(), locked.0, format.data.len());
    GlobalUnlock(handle);
    handle
  })
}

fn with_clipboard<V>(f: impl FnOnce() -> V) -> Option<V> {
  unsafe {
    if !OpenClipboard(HWND::default()).as_bool() {
      return None;
    }

    let result = f();

    CloseClipboard();

    Some(result)
  }
}

// https://docs.microsoft.com/en-ca/windows/win32/dataxchg/standard-clipboard-formats
static STANDARD_FORMATS: &[(u32, &str)] = &[
  (1, "CF_TEXT"),
  (2, "CF_BITMAP"),
  (3, "CF_METAFILEPICT"),
  (4, "CF_SYLK"),
  (5, "CF_DIF"),
  (6, "CF_TIFF"),
  (7, "CF_OEMTEXT"),
  (8, "CF_DIB"),
  (9, "CF_PALETTE"),
  (10, "CF_PENDATA"),
  (11, "CF_RIFF"),
  (12, "CF_WAVE"),
  (13, "CF_UNICODETEXT"),
  (14, "CF_ENHMETAFILE"),
  (15, "CF_HDROP"),
  (16, "CF_LOCALE"),
  (17, "CF_DIBV5"),
  (0x0080, "CF_OWNERDISPLAY"),
  (0x0081, "CF_DSPTEXT"),
  (0x0082, "CF_DSPBITMAP"),
  (0x0083, "CF_DSPMETAFILEPICT"),
  (0x008E, "CF_DSPENHMETAFILE"),
  (0x0200, "CF_PRIVATEFIRST"),
  (0x02FF, "CF_PRIVATELAST"),
  (0x0300, "CF_GDIOBJFIRST"),
  (0x03FF, "CF_GDIOBJLAST"),
];
