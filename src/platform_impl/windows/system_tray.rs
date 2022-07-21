// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use super::{
  event_loop::S_U_TASKBAR_RESTART,
  menu::{subclass_proc as menu_subclass_proc, Menu, MenuHandler},
  util, OsError,
};
use crate::{
  dpi::{PhysicalPosition, PhysicalSize},
  error::OsError as RootOsError,
  event::{Event, Rectangle, TrayEvent},
  event_loop::EventLoopWindowTarget,
  menu::MenuType,
  system_tray::{Icon, SystemTray as RootSystemTray},
};
use windows::{
  core::{PCSTR, PCWSTR},
  Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    System::LibraryLoader::*,
    UI::{
      Shell::*,
      WindowsAndMessaging::{self as win32wm, *},
    },
  },
};

const WM_USER_TRAYICON: u32 = 6001;
const WM_USER_UPDATE_TRAYMENU: u32 = 6002;
const WM_USER_UPDATE_TRAYICON: u32 = 6003;
const TRAYICON_UID: u32 = 6004;
const TRAY_SUBCLASS_ID: usize = 6005;
const TRAY_MENU_SUBCLASS_ID: usize = 6006;

struct TrayLoopData {
  hwnd: HWND,
  hmenu: Option<HMENU>,
  icon: Icon,
  sender: Box<dyn Fn(Event<'static, ()>)>,
}

pub struct SystemTrayBuilder {
  pub(crate) icon: Icon,
  pub(crate) tray_menu: Option<Menu>,
}

impl SystemTrayBuilder {
  #[inline]
  pub fn new(icon: Icon, tray_menu: Option<Menu>) -> Self {
    Self { icon, tray_menu }
  }

  #[inline]
  pub fn build<T: 'static>(
    self,
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<RootSystemTray, RootOsError> {
    let hmenu: Option<HMENU> = self.tray_menu.map(|m| m.hmenu());

    let class_name = util::encode_wide("tao_system_tray_app");
    unsafe {
      let hinstance = GetModuleHandleA(PCSTR::default()).unwrap_or_default();

      let wnd_class = WNDCLASSW {
        lpfnWndProc: Some(util::call_default_window_proc),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        hInstance: hinstance,
        ..Default::default()
      };

      RegisterClassW(&wnd_class);

      let hwnd = CreateWindowExW(
        Default::default(),
        PCWSTR(class_name.as_ptr()),
        "tao_system_tray_window",
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        0,
        CW_USEDEFAULT,
        0,
        HWND::default(),
        HMENU::default(),
        hinstance,
        std::ptr::null_mut(),
      );
      if !IsWindow(hwnd).as_bool() {
        return Err(os_error!(OsError::CreationError(
          "Unable to get valid mutable pointer for CreateWindowEx"
        )));
      }

      let hicon = self.icon.inner.as_raw_handle();

      if !register_tray_icon(hwnd, hicon) {
        return Err(os_error!(OsError::CreationError(
          "Error with shellapi::Shell_NotifyIconW"
        )));
      }

      let system_tray = SystemTray { hwnd: hwnd.clone() };

      // system_tray event handler
      let event_loop_runner = window_target.p.runner_shared.clone();
      let traydata = TrayLoopData {
        hwnd,
        hmenu,
        icon: self.icon,
        sender: Box::new(move |event| {
          if let Ok(e) = event.map_nonuser_event() {
            event_loop_runner.send_event(e)
          }
        }),
      };
      SetWindowSubclass(
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
      SetWindowSubclass(
        hwnd,
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
  pub fn set_icon(&mut self, icon: Icon) {
    unsafe {
      let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_ICON,
        hWnd: self.hwnd,
        hIcon: icon.inner.as_raw_handle(),
        uID: TRAYICON_UID,
        ..std::mem::zeroed()
      };
      if !Shell_NotifyIconW(NIM_MODIFY, &mut nid as _).as_bool() {
        debug!("Error setting icon");
      }

      // send the new icon to the subclass proc to store it in the tray data
      SendMessageW(
        self.hwnd,
        WM_USER_UPDATE_TRAYICON,
        WPARAM(Box::into_raw(Box::new(icon)) as _),
        LPARAM(0),
      );
    }
  }

  pub fn set_menu(&mut self, tray_menu: &Menu) {
    unsafe {
      // send the new menu to the subclass proc where we will update there
      SendMessageW(
        self.hwnd,
        WM_USER_UPDATE_TRAYMENU,
        WPARAM(tray_menu.hmenu().0 as _),
        LPARAM(0),
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
        ..std::mem::zeroed()
      };
      if !Shell_NotifyIconW(NIM_DELETE, &mut nid as _).as_bool() {
        debug!("Error removing system tray icon");
      }

      // destroy the hidden window used by the tray
      DestroyWindow(self.hwnd);
    }
  }
}

unsafe extern "system" fn tray_subclass_proc(
  hwnd: HWND,
  msg: u32,
  wparam: WPARAM,
  lparam: LPARAM,
  _id: usize,
  subclass_input_ptr: usize,
) -> LRESULT {
  let subclass_input_ptr = subclass_input_ptr as *mut TrayLoopData;
  let mut subclass_input = &mut *(subclass_input_ptr);

  if msg == WM_DESTROY {
    Box::from_raw(subclass_input_ptr);
  }

  if msg == WM_USER_UPDATE_TRAYMENU {
    subclass_input.hmenu = Some(HMENU(wparam.0 as _));
  }

  if msg == WM_USER_UPDATE_TRAYICON {
    let icon = wparam.0 as *mut Icon;
    subclass_input.icon = (*icon).clone();
  }

  if msg == *S_U_TASKBAR_RESTART {
    register_tray_icon(
      subclass_input.hwnd,
      subclass_input.icon.inner.as_raw_handle(),
    );
  }

  if msg == WM_USER_TRAYICON
    && matches!(
      lparam.0 as u32,
      WM_LBUTTONUP | WM_RBUTTONUP | WM_LBUTTONDBLCLK
    )
  {
    let nid = NOTIFYICONIDENTIFIER {
      hWnd: hwnd,
      cbSize: std::mem::size_of::<NOTIFYICONIDENTIFIER>() as _,
      uID: TRAYICON_UID,
      ..std::mem::zeroed()
    };
    let icon_rect = Shell_NotifyIconGetRect(&nid).unwrap_or_default();

    let mut cursor = POINT { x: 0, y: 0 };
    GetCursorPos(&mut cursor as _);

    let position = PhysicalPosition::new(cursor.x as f64, cursor.y as f64);
    let bounds = Rectangle {
      position: PhysicalPosition::new(icon_rect.left as f64, icon_rect.top as f64),
      size: PhysicalSize::new(
        (icon_rect.right - icon_rect.left) as f64,
        (icon_rect.bottom - icon_rect.top) as f64,
      ),
    };

    match lparam.0 as u32 {
      win32wm::WM_LBUTTONUP => {
        (subclass_input.sender)(Event::TrayEvent {
          event: TrayEvent::LeftClick,
          position,
          bounds,
        });
      }

      win32wm::WM_RBUTTONUP => {
        (subclass_input.sender)(Event::TrayEvent {
          event: TrayEvent::RightClick,
          position,
          bounds,
        });

        if let Some(menu) = subclass_input.hmenu {
          show_tray_menu(hwnd, menu, cursor.x, cursor.y);
        }
      }

      win32wm::WM_LBUTTONDBLCLK => {
        (subclass_input.sender)(Event::TrayEvent {
          event: TrayEvent::DoubleClick,
          position,
          bounds,
        });
      }

      _ => {}
    }
  }

  DefSubclassProc(hwnd, msg, wparam, lparam)
}

unsafe fn show_tray_menu(hwnd: HWND, menu: HMENU, x: i32, y: i32) {
  // bring the hidden window to the foreground so the pop up menu
  // would automatically hide on click outside
  SetForegroundWindow(hwnd);
  // track the click
  TrackPopupMenu(
    menu,
    // align bottom / right, maybe we could expose this later..
    TPM_BOTTOMALIGN | TPM_LEFTALIGN,
    x,
    y,
    0,
    hwnd,
    std::ptr::null_mut(),
  );
}

unsafe fn register_tray_icon(hwnd: HWND, hicon: HICON) -> bool {
  let mut nid = NOTIFYICONDATAW {
    uFlags: NIF_MESSAGE | NIF_ICON,
    hWnd: hwnd,
    hIcon: hicon,
    uID: TRAYICON_UID,
    uCallbackMessage: WM_USER_TRAYICON,
    ..std::mem::zeroed()
  };

  Shell_NotifyIconW(NIM_ADD, &mut nid as _).as_bool()
}
