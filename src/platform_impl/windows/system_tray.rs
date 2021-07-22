// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use super::{
  dpi::{dpi_to_scale_factor, hwnd_dpi},
  menu::{subclass_proc as menu_subclass_proc, to_wstring, Menu, MenuHandler},
  util, OsError,
};
use crate::{
  dpi::{LogicalPosition, LogicalSize},
  error::OsError as RootOsError,
  event::{Event, Rectangle, TrayEvent},
  event_loop::EventLoopWindowTarget,
  menu::MenuType,
  system_tray::SystemTray as RootSystemTray,
};
use winapi::{
  shared::{
    basetsd::{DWORD_PTR, UINT_PTR},
    minwindef::{LPARAM, LRESULT, UINT, WPARAM},
    windef::{HICON, HMENU, HWND, POINT, RECT},
  },
  um::{
    commctrl, libloaderapi,
    shellapi::{self, NIF_ICON, NIF_MESSAGE, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW},
    winuser::{self, CW_USEDEFAULT, WNDCLASSW, WS_OVERLAPPEDWINDOW},
  },
};

const WM_USER_TRAYICON: u32 = 6001;
const WM_USER_UPDATE_TRAYMENU: u32 = 6002;
const TRAYICON_UID: u32 = 6003;
const TRAY_SUBCLASS_ID: usize = 6004;
const TRAY_MENU_SUBCLASS_ID: usize = 6005;

struct TrayLoopData {
  hmenu: Option<HMENU>,
  sender: Box<dyn Fn(Event<'static, ()>)>,
}

pub struct SystemTrayBuilder {
  pub(crate) icon: Vec<u8>,
  pub(crate) tray_menu: Option<Menu>,
}

impl SystemTrayBuilder {
  #[inline]
  pub fn new(icon: Vec<u8>, tray_menu: Option<Menu>) -> Self {
    Self { icon, tray_menu }
  }

  #[inline]
  pub fn build<T: 'static>(
    self,
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<RootSystemTray, RootOsError> {
    let hmenu: Option<HMENU> = self.tray_menu.map(|m| m.hmenu());

    let class_name = to_wstring("tao_system_tray_app");
    unsafe {
      let hinstance = libloaderapi::GetModuleHandleA(std::ptr::null_mut());

      let wnd_class = WNDCLASSW {
        lpfnWndProc: Some(winuser::DefWindowProcW),
        lpszClassName: class_name.as_ptr(),
        hInstance: hinstance,
        ..Default::default()
      };

      if winuser::RegisterClassW(&wnd_class) == 0 {
        return Err(os_error!(OsError::CreationError(
          "Error with winuser::RegisterClassW"
        )));
      }

      let hwnd = winuser::CreateWindowExW(
        0,
        class_name.as_ptr(),
        to_wstring("tao_system_tray_window").as_ptr(),
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        0,
        CW_USEDEFAULT,
        0,
        0 as _,
        0 as _,
        hinstance as _,
        std::ptr::null_mut(),
      );

      if hwnd.is_null() {
        return Err(os_error!(OsError::CreationError(
          "Unable to get valid mutable pointer for winuser::CreateWindowEx"
        )));
      }

      let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_MESSAGE,
        hWnd: hwnd,
        uID: TRAYICON_UID,
        uCallbackMessage: WM_USER_TRAYICON,
        ..Default::default()
      };

      if shellapi::Shell_NotifyIconW(NIM_ADD, &mut nid as _) == 0 {
        return Err(os_error!(OsError::CreationError(
          "Error with shellapi::Shell_NotifyIconW"
        )));
      }

      let system_tray = SystemTray { hwnd };
      system_tray.set_icon_from_buffer(&self.icon, 32, 32);

      // system_tray event handler
      let event_loop_runner = window_target.p.runner_shared.clone();
      let traydata = TrayLoopData {
        hmenu,
        sender: Box::new(move |event| {
          if let Ok(e) = event.map_nonuser_event() {
            event_loop_runner.send_event(e)
          }
        }),
      };
      commctrl::SetWindowSubclass(
        hwnd,
        Some(tray_subclass_proc),
        TRAY_SUBCLASS_ID,
        Box::into_raw(Box::new(traydata)) as _,
      );

      // system_tray menu event handler
      let event_loop_runner = window_target.p.runner_shared.clone();
      let menu_handler = MenuHandler::new(
        Box::new(move |event| {
          if let Ok(e) = event.map_nonuser_event() {
            event_loop_runner.send_event(e)
          }
        }),
        MenuType::ContextMenu,
        None,
      );
      commctrl::SetWindowSubclass(
        hwnd as _,
        Some(menu_subclass_proc),
        TRAY_MENU_SUBCLASS_ID,
        Box::into_raw(Box::new(menu_handler)) as _,
      );

      Ok(RootSystemTray(system_tray))
    }
  }
}

pub struct SystemTray {
  hwnd: HWND,
}

impl SystemTray {
  pub fn set_icon(&mut self, icon: Vec<u8>) {
    self.set_icon_from_buffer(&icon, 32, 32);
  }

  fn set_icon_from_buffer(&self, buffer: &[u8], width: u32, height: u32) {
    if let Some(hicon) = util::get_hicon_from_buffer(buffer, width as _, height as _) {
      self.set_hicon(hicon);
    }
  }

  fn set_hicon(&self, icon: HICON) {
    unsafe {
      let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_ICON,
        hWnd: self.hwnd,
        hIcon: icon,
        uID: TRAYICON_UID,
        ..Default::default()
      };
      if shellapi::Shell_NotifyIconW(NIM_MODIFY, &mut nid as _) == 0 {
        debug!("Error setting icon");
      }
    }
  }

  pub fn set_menu(&mut self, tray_menu: &Menu) {
    unsafe {
      // send the new menu to the subclass proc where we will update there
      winuser::SendMessageW(
        self.hwnd,
        WM_USER_UPDATE_TRAYMENU,
        tray_menu.hmenu() as _,
        0,
      );
    }
  }
}

impl Drop for SystemTray {
  fn drop(&mut self) {
    unsafe {
      // remove the icon from system tray
      let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_ICON,
        hWnd: self.hwnd,
        uID: TRAYICON_UID,
        ..Default::default()
      };
      if shellapi::Shell_NotifyIconW(NIM_DELETE, &mut nid as _) == 0 {
        debug!("Error removing system tray icon");
      }

      // destroy the hidden window used by the tray
      winuser::DestroyWindow(self.hwnd);
    }
  }
}

unsafe extern "system" fn tray_subclass_proc(
  hwnd: HWND,
  msg: UINT,
  wparam: WPARAM,
  lparam: LPARAM,
  _id: UINT_PTR,
  subclass_input_ptr: DWORD_PTR,
) -> LRESULT {
  let subclass_input_ptr = subclass_input_ptr as *mut TrayLoopData;
  let mut subclass_input = &mut *(subclass_input_ptr);

  if msg == winuser::WM_DESTROY {
    Box::from_raw(subclass_input_ptr);
  }

  if msg == WM_USER_UPDATE_TRAYMENU {
    subclass_input.hmenu = Some(wparam as HMENU);
  }

  if msg == WM_USER_TRAYICON
    && matches!(
      lparam as u32,
      winuser::WM_LBUTTONUP | winuser::WM_RBUTTONUP | winuser::WM_LBUTTONDBLCLK
    )
  {
    let mut icon_rect = RECT::default();
    let nid = shellapi::NOTIFYICONIDENTIFIER {
      hWnd: hwnd,
      cbSize: std::mem::size_of::<shellapi::NOTIFYICONIDENTIFIER>() as _,
      uID: TRAYICON_UID,
      ..Default::default()
    };
    shellapi::Shell_NotifyIconGetRect(&nid, &mut icon_rect);

    let dpi = hwnd_dpi(hwnd);
    let scale_factor = dpi_to_scale_factor(dpi);

    let mut cursor = POINT { x: 0, y: 0 };
    winuser::GetCursorPos(&mut cursor as _);

    let position = LogicalPosition::new(cursor.x, cursor.y).to_physical(scale_factor);
    let bounds = Rectangle {
      position: LogicalPosition::new(icon_rect.left, icon_rect.top).to_physical(scale_factor),
      size: LogicalSize::new(
        icon_rect.right - icon_rect.left,
        icon_rect.bottom - icon_rect.top,
      )
      .to_physical(scale_factor),
    };

    match lparam as u32 {
      winuser::WM_LBUTTONUP => {
        (subclass_input.sender)(Event::TrayEvent {
          event: TrayEvent::LeftClick,
          position,
          bounds,
        });
      }

      winuser::WM_RBUTTONUP => {
        (subclass_input.sender)(Event::TrayEvent {
          event: TrayEvent::RightClick,
          position,
          bounds,
        });

        if let Some(menu) = subclass_input.hmenu {
          show_tray_menu(hwnd, menu, cursor.x, cursor.y);
        }
      }

      winuser::WM_LBUTTONDBLCLK => {
        (subclass_input.sender)(Event::TrayEvent {
          event: TrayEvent::DoubleClick,
          position,
          bounds,
        });
      }

      _ => {}
    }
  }

  commctrl::DefSubclassProc(hwnd, msg, wparam, lparam)
}

unsafe fn show_tray_menu(hwnd: HWND, menu: HMENU, x: i32, y: i32) {
  // bring the hidden window to the foreground so the pop up menu
  // would automatically hide on click outside
  winuser::SetForegroundWindow(hwnd);
  // track the click
  winuser::TrackPopupMenu(
    menu,
    0,
    x,
    y,
    // align bottom / right, maybe we could expose this later..
    (winuser::TPM_BOTTOMALIGN | winuser::TPM_LEFTALIGN) as _,
    hwnd,
    std::ptr::null_mut(),
  );
}
