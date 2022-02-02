// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use super::{menu::subclass_proc as menu_subclass_proc, util, OsError};
use crate::{
  dpi::{PhysicalPosition, PhysicalSize},
  error::OsError as RootOsError,
  event::{Event, Rectangle, TrayEvent},
  event_loop::EventLoopWindowTarget,
  menu::{Menu, MENUS_DATA},
  system_tray::SystemTray as RootSystemTray,
  window::Icon,
};
use windows::Win32::{
  Foundation::{HWND, LPARAM, LRESULT, POINT, PSTR, PWSTR, WPARAM},
  System::LibraryLoader::*,
  UI::{
    Shell::*,
    WindowsAndMessaging::{self as win32wm, *},
  },
};

const WM_USER_TRAYICON: u32 = 6001;
const WM_USER_UPDATE_TRAYMENU: u32 = 6002;
const TRAYICON_UID: u32 = 6003;
const TRAY_SUBCLASS_ID: usize = 6004;
const TRAY_MENU_SUBCLASS_ID: usize = 6005;

struct TrayLoopData {
  menu: Option<Menu>,
  sender: Box<dyn Fn(Event<'static, ()>)>,
}

pub struct SystemTrayBuilder {
  pub(crate) icon: Icon,
  pub(crate) menu: Option<Menu>,
}

impl SystemTrayBuilder {
  #[inline]
  pub fn new(icon: Icon, menu: Option<Menu>) -> Self {
    Self { icon, menu }
  }

  #[inline]
  pub fn build<T: 'static>(
    self,
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<RootSystemTray, RootOsError> {
    let mut class_name = util::to_wstring("tao_system_tray_app");
    unsafe {
      let hinstance = GetModuleHandleA(PSTR::default());

      let wnd_class = WNDCLASSW {
        lpfnWndProc: Some(util::call_default_window_proc),
        lpszClassName: PWSTR(class_name.as_mut_ptr()),
        hInstance: hinstance,
        ..Default::default()
      };

      RegisterClassW(&wnd_class);

      let hwnd = CreateWindowExW(
        0,
        PWSTR(class_name.as_mut_ptr()),
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

      if hwnd.is_invalid() {
        return Err(os_error!(OsError::CreationError(
          "Unable to get valid mutable pointer for CreateWindowEx"
        )));
      }

      let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_MESSAGE,
        hWnd: hwnd,
        uID: TRAYICON_UID,
        uCallbackMessage: WM_USER_TRAYICON,
        ..std::mem::zeroed()
      };

      if !Shell_NotifyIconW(NIM_ADD, &mut nid as _).as_bool() {
        return Err(os_error!(OsError::CreationError(
          "Error with shellapi::Shell_NotifyIconW"
        )));
      }

      let mut system_tray = SystemTray {
        window_target: hwnd,
      };
      system_tray.set_icon(self.icon);

      let event_loop_runner = window_target.p.runner_shared.clone();
      let traydata = TrayLoopData {
        menu: self.menu,
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

      SetWindowSubclass(hwnd, Some(menu_subclass_proc), TRAY_MENU_SUBCLASS_ID, 0);

      Ok(RootSystemTray(system_tray))
    }
  }
}

pub struct SystemTray {
  window_target: HWND,
}

impl SystemTray {
  pub fn set_icon(&mut self, icon: Icon) {
    unsafe {
      let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_ICON,
        hWnd: self.window_target,
        hIcon: icon.inner.as_raw_handle(),
        uID: TRAYICON_UID,
        ..std::mem::zeroed()
      };
      if !Shell_NotifyIconW(NIM_MODIFY, &mut nid as _).as_bool() {
        debug!("Error setting icon");
      }
    }
  }

  pub fn set_menu(&mut self, menu: Option<Menu>) {
    // send the new menu to the subclass proc where we can update the TrayLoopData
    unsafe {
      SendMessageW(
        self.window_target,
        WM_USER_UPDATE_TRAYMENU,
        if let Some(m) = menu {
          WPARAM(m.0 as _)
        } else {
          WPARAM::default()
        },
        LPARAM::default(),
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
        hWnd: self.window_target,
        uID: TRAYICON_UID,
        ..std::mem::zeroed()
      };
      if !Shell_NotifyIconW(NIM_DELETE, &mut nid as _).as_bool() {
        debug!("Error removing system tray icon");
      }

      // destroy the hidden window used by the tray
      DestroyWindow(self.window_target);
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
  let mut tray_data = &mut *(subclass_input_ptr);

  if msg == WM_DESTROY {
    Box::from_raw(subclass_input_ptr);
  }

  if msg == WM_USER_UPDATE_TRAYMENU {
    tray_data.menu = if !wparam.is_invalid() {
      Some(Menu(wparam.0 as _))
    } else {
      None
    };
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
        (tray_data.sender)(Event::TrayEvent {
          event: TrayEvent::LeftClick,
          position,
          bounds,
        });
      }

      win32wm::WM_RBUTTONUP => {
        (tray_data.sender)(Event::TrayEvent {
          event: TrayEvent::RightClick,
          position,
          bounds,
        });

        if let Some(menu) = tray_data.menu.clone() {
          let context_menu = CreatePopupMenu();
          if let Ok(menus_data) = MENUS_DATA.lock() {
            if let Some(menu) = menus_data.menus.get(&menu.id()) {
              menu.add_items_to_hmenu(context_menu, &menus_data);
            }
          }
          util::show_context_menu(hwnd, context_menu, cursor.x, cursor.y);
          DestroyMenu(context_menu);
        }
      }

      win32wm::WM_LBUTTONDBLCLK => {
        (tray_data.sender)(Event::TrayEvent {
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
