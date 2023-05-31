use std::ffi::CStr;

use lazy_static::lazy_static;
use raw_window_handle::{
  RawDisplayHandle, RawWindowHandle, XcbDisplayHandle, XcbWindowHandle, XlibDisplayHandle,
  XlibWindowHandle,
};
use x11_dl::xlib::{self, PropModeReplace, Xlib};
use x11rb::{
  protocol::xproto::{AtomEnum, ConnectionExt, PropMode},
  wrapper::ConnectionExt as WrapperConnectionExt,
  xcb_ffi::XCBConnection,
};

lazy_static! {
  static ref XLIB: Xlib = Xlib::open().unwrap();
}

pub struct Manager {
  handle: (RawWindowHandle, RawDisplayHandle),
  progress: u32,
  progress_visible: bool,
  pulse: bool,
}

impl Manager {
  pub fn new(handle: RawWindowHandle, d_handle: RawDisplayHandle) -> Option<Self> {
    if !matches!(handle, RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_)) {
      return None;
    }
    Some(Self {
      handle: (handle, d_handle),
      progress: 0,
      progress_visible: false,
      pulse: false,
    })
  }

  pub fn update_property(&self, name: &CStr, value: u32) {
    match self.handle {
      (
        RawWindowHandle::Xlib(XlibWindowHandle { window, .. }),
        RawDisplayHandle::Xlib(XlibDisplayHandle { display, .. }),
      ) => {
        let display = display as *mut xlib::Display;
        let atom = unsafe { (XLIB.XInternAtom)(display, name.as_ptr(), xlib::True) };
        unsafe {
          (XLIB.XChangeProperty)(
            display,
            window,
            atom,
            xlib::XA_CARDINAL,
            32,
            PropModeReplace,
            &value as *const _ as *const u8,
            1,
          );
        }
      }
      (
        RawWindowHandle::Xcb(XcbWindowHandle { window, .. }),
        RawDisplayHandle::Xcb(XcbDisplayHandle { connection, .. }),
      ) => {
        let connection =
          unsafe { XCBConnection::from_raw_xcb_connection(connection, false).unwrap() };
        let atom = connection
          .intern_atom(false, name.to_bytes_with_nul())
          .unwrap()
          .reply()
          .unwrap()
          .atom;
        connection
          .change_property32(
            PropMode::REPLACE,
            window,
            atom,
            AtomEnum::CARDINAL,
            &[value],
          )
          .unwrap();
      }
      _ => unreachable!(),
    }
  }

  pub fn delete_property(&self, name: &CStr) {
    match self.handle {
      (
        RawWindowHandle::Xlib(XlibWindowHandle { window, .. }),
        RawDisplayHandle::Xlib(XlibDisplayHandle { display, .. }),
      ) => {
        let display = display as *mut xlib::Display;
        let atom = unsafe { (XLIB.XInternAtom)(display, name.as_ptr(), xlib::True) };
        unsafe {
          (XLIB.XDeleteProperty)(display, window, atom);
        }
      }
      (
        RawWindowHandle::Xcb(XcbWindowHandle { window, .. }),
        RawDisplayHandle::Xcb(XcbDisplayHandle { connection, .. }),
      ) => {
        let connection =
          unsafe { XCBConnection::from_raw_xcb_connection(connection, false).unwrap() };
        let atom = connection
          .intern_atom(false, name.to_bytes_with_nul())
          .unwrap()
          .reply()
          .unwrap()
          .atom;
        connection.delete_property(window, atom).unwrap();
      }
      _ => unreachable!(),
    }
  }

  fn update_progress_property(&self) {
    self.update_property(
      CStr::from_bytes_with_nul(b"_NET_WM_XAPP_PROGRESS\0").unwrap(),
      self.progress,
    );
  }

  pub fn set_progress(&mut self, progress: f64) -> Result<(), Box<dyn std::error::Error>> {
    let progress = (progress * 100.0) as u32;
    if self.progress != progress {
      self.progress = progress;
      if self.progress_visible {
        self.update_progress_property();
      }
    }
    Ok(())
  }

  pub fn set_progress_visible(
    &mut self,
    is_visible: bool,
  ) -> Result<(), Box<dyn std::error::Error>> {
    if self.progress_visible != is_visible {
      self.progress_visible = is_visible;
      if self.progress_visible {
        self.update_progress_property();
      } else {
        self.delete_property(CStr::from_bytes_with_nul(b"_NET_WM_XAPP_PROGRESS\0").unwrap());
      }
    }
    Ok(())
  }

  pub fn needs_attention(
    &mut self,
    needs_attention: bool,
  ) -> Result<(), Box<dyn std::error::Error>> {
    if self.pulse != needs_attention {
      self.pulse = needs_attention;
      self.update_property(
        CStr::from_bytes_with_nul(b"_NET_WM_XAPP_PROGRESS_PULSE\0").unwrap(),
        needs_attention as u32,
      );
    }
    Ok(())
  }
}
