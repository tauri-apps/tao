// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  io, mem,
  ops::BitAnd,
  os::windows::prelude::OsStrExt,
  ptr, slice,
  sync::atomic::{AtomicBool, Ordering},
};

use crate::{dpi::PhysicalSize, window::CursorIcon};

use windows::{
  core::HRESULT,
  Win32::{
    Foundation::{BOOL, FARPROC, HWND, LPARAM, LRESULT, POINT, PWSTR, RECT, WPARAM},
    Globalization::lstrlenW,
    Graphics::Gdi::{ClientToScreen, InvalidateRgn, HMONITOR, HRGN},
    System::LibraryLoader::*,
    UI::{
      HiDpi::*,
      Input::KeyboardAndMouse::*,
      TextServices::HKL,
      WindowsAndMessaging::{self as win32wm, *},
    },
  },
};

pub fn has_flag<T>(bitset: T, flag: T) -> bool
where
  T: Copy + PartialEq + BitAnd<T, Output = T>,
{
  bitset & flag == flag
}

pub fn wchar_to_string(wchar: &[u16]) -> String {
  String::from_utf16_lossy(wchar)
}

pub fn wchar_ptr_to_string(wchar: PWSTR) -> String {
  let len = unsafe { lstrlenW(wchar) } as usize;
  let wchar_slice = unsafe { slice::from_raw_parts(wchar.0, len) };
  wchar_to_string(wchar_slice)
}

pub fn to_wstring(str: &str) -> Vec<u16> {
  std::ffi::OsStr::new(str)
    .encode_wide()
    .chain(Some(0).into_iter())
    .collect()
}

pub unsafe fn status_map<T, F: FnMut(&mut T) -> BOOL>(mut fun: F) -> Option<T> {
  let mut data: T = mem::zeroed();
  if fun(&mut data).as_bool() {
    Some(data)
  } else {
    None
  }
}

fn win_to_err<F: FnOnce() -> BOOL>(f: F) -> Result<(), io::Error> {
  if f().as_bool() {
    Ok(())
  } else {
    Err(io::Error::last_os_error())
  }
}

pub fn get_window_rect(hwnd: HWND) -> Option<RECT> {
  unsafe { status_map(|rect| GetWindowRect(hwnd, rect)) }
}

pub fn get_client_rect(hwnd: HWND) -> Result<RECT, io::Error> {
  let mut rect = RECT::default();
  let mut top_left = POINT::default();

  unsafe {
    win_to_err(|| ClientToScreen(hwnd, &mut top_left))?;
    win_to_err(|| GetClientRect(hwnd, &mut rect))?;
  }

  rect.left += top_left.x;
  rect.top += top_left.y;
  rect.right += top_left.x;
  rect.bottom += top_left.y;

  Ok(rect)
}

pub fn adjust_size(hwnd: HWND, size: PhysicalSize<u32>) -> PhysicalSize<u32> {
  let (width, height): (u32, u32) = size.into();
  let rect = RECT {
    left: 0,
    right: width as i32,
    top: 0,
    bottom: height as i32,
  };
  let rect = adjust_window_rect(hwnd, rect).unwrap_or(rect);
  PhysicalSize::new((rect.right - rect.left) as _, (rect.bottom - rect.top) as _)
}

pub(crate) fn set_inner_size_physical(window: HWND, x: u32, y: u32) {
  unsafe {
    let rect = adjust_window_rect(
      window,
      RECT {
        top: 0,
        left: 0,
        bottom: y as i32,
        right: x as i32,
      },
    )
    .expect("adjust_window_rect failed");

    let outer_x = (rect.right - rect.left).abs();
    let outer_y = (rect.top - rect.bottom).abs();
    SetWindowPos(
      window,
      HWND::default(),
      0,
      0,
      outer_x,
      outer_y,
      SWP_ASYNCWINDOWPOS | SWP_NOZORDER | SWP_NOREPOSITION | SWP_NOMOVE | SWP_NOACTIVATE,
    );
    InvalidateRgn(window, HRGN::default(), BOOL::default());
  }
}

pub fn adjust_window_rect(hwnd: HWND, rect: RECT) -> Option<RECT> {
  unsafe {
    let style = GetWindowLongW(hwnd, GWL_STYLE) as WINDOW_STYLE;
    let style_ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as WINDOW_EX_STYLE;
    adjust_window_rect_with_styles(hwnd, style, style_ex, rect)
  }
}

pub fn adjust_window_rect_with_styles(
  hwnd: HWND,
  style: WINDOW_STYLE,
  style_ex: WINDOW_EX_STYLE,
  rect: RECT,
) -> Option<RECT> {
  unsafe {
    status_map(|r| {
      *r = rect;

      let b_menu: BOOL = (!GetMenu(hwnd).is_invalid()).into();

      if let (Some(get_dpi_for_window), Some(adjust_window_rect_ex_for_dpi)) =
        (*GET_DPI_FOR_WINDOW, *ADJUST_WINDOW_RECT_EX_FOR_DPI)
      {
        let dpi = get_dpi_for_window(hwnd);
        adjust_window_rect_ex_for_dpi(r, style, b_menu, style_ex, dpi)
      } else {
        AdjustWindowRectEx(r, style, b_menu, style_ex)
      }
    })
  }
}

pub fn set_cursor_hidden(hidden: bool) {
  static HIDDEN: AtomicBool = AtomicBool::new(false);
  let changed = HIDDEN.swap(hidden, Ordering::SeqCst) ^ hidden;
  if changed {
    unsafe { ShowCursor(!hidden) };
  }
}

pub fn get_cursor_clip() -> Result<RECT, io::Error> {
  unsafe {
    let mut rect = RECT::default();
    win_to_err(|| GetClipCursor(&mut rect)).map(|_| rect)
  }
}

/// Sets the cursor's clip rect.
///
/// Note that calling this will automatically dispatch a `WM_MOUSEMOVE` event.
pub fn set_cursor_clip(rect: Option<RECT>) -> Result<(), io::Error> {
  unsafe {
    let rect_ptr = rect
      .as_ref()
      .map(|r| r as *const RECT)
      .unwrap_or(ptr::null());
    win_to_err(|| ClipCursor(rect_ptr))
  }
}

pub fn get_desktop_rect() -> RECT {
  unsafe {
    let left = GetSystemMetrics(SM_XVIRTUALSCREEN);
    let top = GetSystemMetrics(SM_YVIRTUALSCREEN);
    RECT {
      left,
      top,
      right: left + GetSystemMetrics(SM_CXVIRTUALSCREEN),
      bottom: top + GetSystemMetrics(SM_CYVIRTUALSCREEN),
    }
  }
}

pub fn is_focused(window: HWND) -> bool {
  window == unsafe { GetActiveWindow() }
}

pub fn is_visible(window: HWND) -> bool {
  unsafe { IsWindowVisible(window).as_bool() }
}

pub fn is_maximized(window: HWND) -> bool {
  let mut placement = WINDOWPLACEMENT {
    length: mem::size_of::<WINDOWPLACEMENT>() as u32,
    ..WINDOWPLACEMENT::default()
  };
  unsafe {
    GetWindowPlacement(window, &mut placement);
  }
  placement.showCmd == SW_MAXIMIZE
}

pub fn get_hicon_from_buffer(buffer: &[u8], width: i32, height: i32) -> Option<HICON> {
  unsafe {
    match LookupIconIdFromDirectoryEx(buffer.as_ptr() as _, true, width, height, LR_DEFAULTCOLOR)
      as isize
    {
      0 => {
        debug!("Unable to LookupIconIdFromDirectoryEx");
        None
      }
      offset => {
        // once we got the pointer offset for the directory
        // lets create our resource
        match CreateIconFromResourceEx(
          buffer.as_ptr().offset(offset) as _,
          buffer.len() as _,
          true,
          0x00030000,
          0,
          0,
          LR_DEFAULTCOLOR,
        )
        .ok()
        {
          // windows is really tough on icons
          // if a bad icon is provided it'll fail here or in
          // the LookupIconIdFromDirectoryEx if this is a bad format (example png's)
          // with my tests, even some ICO's were failing...
          Err(err) => {
            debug!("Unable to CreateIconFromResourceEx: {:?}", err);
            None
          }
          Ok(hicon) => Some(hicon),
        }
      }
    }
  }
}

impl CursorIcon {
  pub(crate) fn to_windows_cursor(self) -> PWSTR {
    match self {
      CursorIcon::Arrow | CursorIcon::Default => IDC_ARROW,
      CursorIcon::Hand => IDC_HAND,
      CursorIcon::Crosshair => IDC_CROSS,
      CursorIcon::Text | CursorIcon::VerticalText => IDC_IBEAM,
      CursorIcon::NotAllowed | CursorIcon::NoDrop => IDC_NO,
      CursorIcon::Grab | CursorIcon::Grabbing | CursorIcon::Move | CursorIcon::AllScroll => {
        IDC_SIZEALL
      }
      CursorIcon::EResize | CursorIcon::WResize | CursorIcon::EwResize | CursorIcon::ColResize => {
        IDC_SIZEWE
      }
      CursorIcon::NResize | CursorIcon::SResize | CursorIcon::NsResize | CursorIcon::RowResize => {
        IDC_SIZENS
      }
      CursorIcon::NeResize | CursorIcon::SwResize | CursorIcon::NeswResize => IDC_SIZENESW,
      CursorIcon::NwResize | CursorIcon::SeResize | CursorIcon::NwseResize => IDC_SIZENWSE,
      CursorIcon::Wait => IDC_WAIT,
      CursorIcon::Progress => IDC_APPSTARTING,
      CursorIcon::Help => IDC_HELP,
      _ => IDC_ARROW, // use arrow for the missing cases.
    }
  }
}

// Helper function to dynamically load function pointer.
// `library` and `function` must be zero-terminated.
pub(super) fn get_function_impl(library: &str, function: &str) -> FARPROC {
  assert_eq!(library.chars().last(), Some('\0'));
  assert_eq!(function.chars().last(), Some('\0'));

  // Library names we will use are ASCII so we can use the A version to avoid string conversion.
  let module = unsafe { LoadLibraryA(library) };
  if module.is_invalid() {
    return None;
  }

  unsafe { GetProcAddress(module, function) }
}

macro_rules! get_function {
  ($lib:expr, $func:ident) => {
    crate::platform_impl::platform::util::get_function_impl(
      concat!($lib, '\0'),
      concat!(stringify!($func), '\0'),
    )
    .map(|f| unsafe { std::mem::transmute::<_, $func>(f) })
  };
}

pub type SetProcessDPIAware = unsafe extern "system" fn() -> BOOL;
pub type SetProcessDpiAwareness =
  unsafe extern "system" fn(value: PROCESS_DPI_AWARENESS) -> HRESULT;
pub type SetProcessDpiAwarenessContext =
  unsafe extern "system" fn(value: DPI_AWARENESS_CONTEXT) -> BOOL;
pub type GetDpiForWindow = unsafe extern "system" fn(hwnd: HWND) -> u32;
pub type GetDpiForMonitor = unsafe extern "system" fn(
  hmonitor: HMONITOR,
  dpi_type: MONITOR_DPI_TYPE,
  dpi_x: *mut u32,
  dpi_y: *mut u32,
) -> HRESULT;
pub type EnableNonClientDpiScaling = unsafe extern "system" fn(hwnd: HWND) -> BOOL;
pub type AdjustWindowRectExForDpi = unsafe extern "system" fn(
  rect: *mut RECT,
  dwStyle: WINDOW_STYLE,
  bMenu: BOOL,
  dwExStyle: WINDOW_EX_STYLE,
  dpi: u32,
) -> BOOL;

lazy_static! {
  pub static ref GET_DPI_FOR_WINDOW: Option<GetDpiForWindow> =
    get_function!("user32.dll", GetDpiForWindow);
  pub static ref ADJUST_WINDOW_RECT_EX_FOR_DPI: Option<AdjustWindowRectExForDpi> =
    get_function!("user32.dll", AdjustWindowRectExForDpi);
  pub static ref GET_DPI_FOR_MONITOR: Option<GetDpiForMonitor> =
    get_function!("shcore.dll", GetDpiForMonitor);
  pub static ref ENABLE_NON_CLIENT_DPI_SCALING: Option<EnableNonClientDpiScaling> =
    get_function!("user32.dll", EnableNonClientDpiScaling);
  pub static ref SET_PROCESS_DPI_AWARENESS_CONTEXT: Option<SetProcessDpiAwarenessContext> =
    get_function!("user32.dll", SetProcessDpiAwarenessContext);
  pub static ref SET_PROCESS_DPI_AWARENESS: Option<SetProcessDpiAwareness> =
    get_function!("shcore.dll", SetProcessDpiAwareness);
  pub static ref SET_PROCESS_DPI_AWARE: Option<SetProcessDPIAware> =
    get_function!("user32.dll", SetProcessDPIAware);
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "32")]
pub fn SetWindowLongPtrW(window: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
  unsafe { win32wm::SetWindowLongW(window, index, value as _) as _ }
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "64")]
pub fn SetWindowLongPtrW(window: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
  unsafe { win32wm::SetWindowLongPtrW(window, index, value) }
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "32")]
pub fn GetWindowLongPtrW(window: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
  unsafe { win32wm::GetWindowLongW(window, index) as _ }
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "64")]
pub fn GetWindowLongPtrW(window: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
  unsafe { win32wm::GetWindowLongPtrW(window, index) }
}

/// Implementation of the `LOWORD` macro.
#[allow(non_snake_case)]
#[inline]
pub fn LOWORD(dword: u32) -> u16 {
  (dword & 0xFFFF) as u16
}

/// Implementation of the `HIWORD` macro.
#[allow(non_snake_case)]
#[inline]
pub fn HIWORD(dword: u32) -> u16 {
  ((dword & 0xFFFF_0000) >> 16) as u16
}

/// Implementation of the `GET_X_LPARAM` macro.
#[allow(non_snake_case)]
#[inline]
pub fn GET_X_LPARAM(lparam: LPARAM) -> i16 {
  ((lparam.0 as usize) & 0xFFFF) as u16 as i16
}

/// Implementation of the `GET_Y_LPARAM` macro.
#[allow(non_snake_case)]
#[inline]
pub fn GET_Y_LPARAM(lparam: LPARAM) -> i16 {
  (((lparam.0 as usize) & 0xFFFF_0000) >> 16) as u16 as i16
}

/// Implementation of the `MAKELPARAM` macro.
/// Inverse of [GET_X_LPARAM] and [GET_Y_LPARAM] to put the (`x`, `y`) signed
/// coordinates/values back into an [LPARAM].
#[allow(non_snake_case)]
#[inline]
pub fn MAKELPARAM(x: i16, y: i16) -> LPARAM {
  LPARAM(((x as u16 as u32) | ((y as u16 as u32) << 16)) as usize as _)
}

/// Implementation of the `GET_WHEEL_DELTA_WPARAM` macro.
#[allow(non_snake_case)]
#[inline]
pub fn GET_WHEEL_DELTA_WPARAM(wparam: WPARAM) -> i16 {
  ((wparam.0 & 0xFFFF_0000) >> 16) as u16 as i16
}

/// Implementation of the `GET_XBUTTON_WPARAM` macro.
#[allow(non_snake_case)]
#[inline]
pub fn GET_XBUTTON_WPARAM(wparam: WPARAM) -> u16 {
  ((wparam.0 & 0xFFFF_0000) >> 16) as u16
}

/// Implementation of the `PRIMARYLANGID` macro.
#[allow(non_snake_case)]
#[inline]
pub fn PRIMARYLANGID(hkl: HKL) -> u32 {
  ((hkl.0 as usize) & 0x3FF) as u32
}

pub unsafe extern "system" fn call_default_window_proc(
  hwnd: HWND,
  msg: u32,
  wparam: WPARAM,
  lparam: LPARAM,
) -> LRESULT {
  DefWindowProcW(hwnd, msg, wparam, lparam)
}
