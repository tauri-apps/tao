// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf, ptr};

use windows::{
  self as Windows,
  Win32::{
    Foundation::{self as win32f, HWND, POINTL, PWSTR},
    System::{
      Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL},
      Ole::{DROPEFFECT_COPY, DROPEFFECT_NONE},
      SystemServices::CF_HDROP,
    },
    UI::Shell::{DragFinish, DragQueryFileW, HDROP},
  },
};

use windows_macros::implement;

use crate::platform_impl::platform::WindowId;

use crate::{event::Event, window::WindowId as SuperWindowId};

#[implement(Windows::Win32::System::Ole::IDropTarget)]
pub struct FileDropHandler {
  window: HWND,
  send_event: Box<dyn Fn(Event<'static, ()>)>,
  cursor_effect: u32,
  hovered_is_valid: bool, /* If the currently hovered item is not valid there must not be any `HoveredFileCancelled` emitted */
}

#[allow(non_snake_case)]
impl FileDropHandler {
  pub fn new(window: HWND, send_event: Box<dyn Fn(Event<'static, ()>)>) -> FileDropHandler {
    Self {
      window,
      send_event,
      cursor_effect: DROPEFFECT_NONE,
      hovered_is_valid: false,
    }
  }

  unsafe fn DragEnter(
    &mut self,
    pDataObj: &Option<IDataObject>,
    _grfKeyState: u32,
    _pt: POINTL,
    pdwEffect: *mut u32,
  ) -> windows::core::Result<()> {
    use crate::event::WindowEvent::HoveredFile;
    let hdrop = Self::iterate_filenames(pDataObj, |filename| {
      (self.send_event)(Event::WindowEvent {
        window_id: SuperWindowId(WindowId(self.window.0)),
        event: HoveredFile(filename),
      });
    });
    self.hovered_is_valid = hdrop.is_some();
    self.cursor_effect = if self.hovered_is_valid {
      DROPEFFECT_COPY
    } else {
      DROPEFFECT_NONE
    };
    *pdwEffect = self.cursor_effect;
    Ok(())
  }

  unsafe fn DragOver(
    &self,
    _grfKeyState: u32,
    _pt: POINTL,
    pdwEffect: *mut u32,
  ) -> windows::core::Result<()> {
    *pdwEffect = self.cursor_effect;
    Ok(())
  }

  unsafe fn DragLeave(&self) -> windows::core::Result<()> {
    use crate::event::WindowEvent::HoveredFileCancelled;
    if self.hovered_is_valid {
      (self.send_event)(Event::WindowEvent {
        window_id: SuperWindowId(WindowId(self.window.0)),
        event: HoveredFileCancelled,
      });
    }
    Ok(())
  }

  unsafe fn Drop(
    &self,
    pDataObj: &Option<IDataObject>,
    _grfKeyState: u32,
    _pt: POINTL,
    _pdwEffect: *mut u32,
  ) -> windows::core::Result<()> {
    use crate::event::WindowEvent::DroppedFile;
    let hdrop = Self::iterate_filenames(pDataObj, |filename| {
      (self.send_event)(Event::WindowEvent {
        window_id: SuperWindowId(WindowId(self.window.0)),
        event: DroppedFile(filename),
      });
    });
    if let Some(hdrop) = hdrop {
      DragFinish(hdrop);
    }
    Ok(())
  }

  unsafe fn iterate_filenames<F>(data_obj: &Option<IDataObject>, callback: F) -> Option<HDROP>
  where
    F: Fn(PathBuf),
  {
    let drop_format = FORMATETC {
      cfFormat: CF_HDROP as u16,
      ptd: ptr::null_mut(),
      dwAspect: DVASPECT_CONTENT as u32,
      lindex: -1,
      tymed: TYMED_HGLOBAL as u32,
    };

    match data_obj
      .as_ref()
      .expect("Received null IDataObject")
      .GetData(&drop_format)
    {
      Ok(medium) => {
        let hglobal = medium.Anonymous.hGlobal;
        let hdrop = HDROP(hglobal);

        // The second parameter (0xFFFFFFFF) instructs the function to return the item count
        let item_count = DragQueryFileW(hdrop, 0xFFFFFFFF, PWSTR::default(), 0);

        for i in 0..item_count {
          // Get the length of the path string NOT including the terminating null character.
          // Previously, this was using a fixed size array of MAX_PATH length, but the
          // Windows API allows longer paths under certain circumstances.
          let character_count = DragQueryFileW(hdrop, i, PWSTR::default(), 0) as usize;
          let str_len = character_count + 1;

          // Fill path_buf with the null-terminated file name
          let mut path_buf = Vec::with_capacity(str_len);
          DragQueryFileW(hdrop, i, PWSTR(path_buf.as_mut_ptr()), str_len as u32);
          path_buf.set_len(str_len);

          callback(OsString::from_wide(&path_buf[0..character_count]).into());
        }

        Some(hdrop)
      }
      Err(error) => {
        debug!(
          "{}",
          match error.code() {
            win32f::DV_E_FORMATETC => {
              // If the dropped item is not a file this error will occur.
              // In this case it is OK to return without taking further action.
              "Error occured while processing dropped/hovered item: item is not a file."
            }
            _ => "Unexpected error occured while processing dropped/hovered item.",
          }
        );
        None
      }
    }
  }
}
