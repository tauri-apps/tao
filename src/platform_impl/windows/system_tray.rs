// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::cell::RefCell;

use super::{
  dpi::{dpi_to_scale_factor, hwnd_dpi},
  menu::{subclass_proc, to_wstring, Menu, MenuHandler},
  util, OsError,
};
use crate::{
  dpi::{LogicalPosition, LogicalSize},
  error::OsError as RootOsError,
  event::{Event, Rectangle, TrayEvent},
  event_loop::EventLoopWindowTarget,
  menu::{ContextMenu, MenuType},
  system_tray::SystemTray as RootSystemTray,
};
use winapi::{
  shared::{
    minwindef::{LPARAM, LRESULT, UINT, WPARAM},
    windef::{HICON, HMENU, HWND, POINT, RECT},
  },
  um::{
    commctrl::SetWindowSubclass,
    libloaderapi,
    shellapi::{self, NIF_ICON, NIF_MESSAGE, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW},
    winuser::{self, CW_USEDEFAULT, WNDCLASSW, WS_OVERLAPPEDWINDOW},
  },
};

const WM_USER_TRAYICON: u32 = 0x400 + 1111;
const WM_USER_TRAYICON_UID: u32 = 0x855 + 1111;
thread_local!(static SYSTEM_TRAY_STASH: RefCell<Option<WindowsLoopData>> = RefCell::new(None));

pub struct SystemTrayBuilder {
  pub(crate) icon: Vec<u8>,
  pub(crate) tray_menu: Option<Menu>,
}

impl SystemTrayBuilder {
  /// Creates a new SystemTray for platforms where this is appropriate.
  /// ## Platform-specific
  ///
  /// - **macOS / Windows:**: receive icon as bytes (`Vec<u8>`)
  /// - **Linux:**: receive icon's path (`PathBuf`)
  #[inline]
  pub fn new(icon: Vec<u8>, tray_menu: Option<Menu>) -> Self {
    Self { icon, tray_menu }
  }

  /// Builds the system tray.
  ///
  /// Possible causes of error include denied permission, incompatible system, and lack of memory.
  #[inline]
  pub fn build<T: 'static>(
    self,
    window_target: &EventLoopWindowTarget<T>,
  ) -> Result<RootSystemTray, RootOsError> {
    let mut hmenu: Option<HMENU> = None;
    if let Some(menu) = self.tray_menu {
      hmenu = Some(menu.into_hmenu());
    }

    let class_name = to_wstring("tao_system_tray_app");
    unsafe {
      let hinstance = libloaderapi::GetModuleHandleA(std::ptr::null_mut());
      let wnd_class = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: winuser::LoadIconW(hinstance, winuser::IDI_APPLICATION),
        hCursor: winuser::LoadCursorW(hinstance, winuser::IDI_APPLICATION),
        hbrBackground: 16 as _,
        lpszMenuName: 0 as _,
        lpszClassName: class_name.as_ptr(),
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

      if hwnd == std::ptr::null_mut() {
        return Err(os_error!(OsError::CreationError(
          "Unable to get valid mutable pointer for winuser::CreateWindowEx"
        )));
      }

      let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_MESSAGE,
        hWnd: hwnd,
        uID: WM_USER_TRAYICON_UID,
        uCallbackMessage: WM_USER_TRAYICON,
        ..Default::default()
      };

      if shellapi::Shell_NotifyIconW(NIM_ADD, &mut nid as _) == 0 {
        return Err(os_error!(OsError::CreationError(
          "Error with shellapi::Shell_NotifyIconW"
        )));
      }

      let app_system_tray = SystemTray { hwnd, hmenu };
      app_system_tray.set_icon_from_buffer(&self.icon, 32, 32);

      // system tray handler
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

      SYSTEM_TRAY_STASH.with(|stash| {
        let data = WindowsLoopData {
          system_tray: SystemTray { hwnd, hmenu },
          sender: menu_handler,
        };
        (*stash.borrow_mut()) = Some(data);
      });

      // create the handler for tray menu events
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

      let sender: *mut MenuHandler = Box::into_raw(Box::new(menu_handler));
      SetWindowSubclass(hwnd as _, Some(subclass_proc), 0, sender as _);

      return Ok(RootSystemTray(app_system_tray));
    }
  }
}

pub struct SystemTray {
  hwnd: HWND,
  hmenu: Option<HMENU>,
}

struct WindowsLoopData {
  system_tray: SystemTray,
  sender: MenuHandler,
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

  // set the icon for our main instance
  fn set_hicon(&self, icon: HICON) {
    unsafe {
      let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_ICON,
        hWnd: self.hwnd,
        hIcon: icon,
        uID: WM_USER_TRAYICON_UID,
        ..Default::default()
      };
      if shellapi::Shell_NotifyIconW(NIM_MODIFY, &mut nid as _) == 0 {
        debug!("Error setting icon");
      }
    }
  }

  pub fn remove(&self) {
    unsafe {
      let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_ICON,
        hWnd: self.hwnd,
        uID: WM_USER_TRAYICON_UID,
        ..Default::default()
      };
      if shellapi::Shell_NotifyIconW(NIM_DELETE, &mut nid as _) == 0 {
        debug!("Error removing icon");
      }
    }
  }

  pub fn set_menu(&mut self, tray_menu: &Menu) {
    let new_menu = Some(tray_menu.hmenu);
    self.hmenu = new_menu;
    SYSTEM_TRAY_STASH.with(|stash| {
      if let Some(ref mut data) = *stash.borrow_mut() {
        data.system_tray.hmenu = new_menu;
      }
    });
  }
}

unsafe extern "system" fn window_proc(
  hwnd: HWND,
  msg: UINT,
  wparam: WPARAM,
  lparam: LPARAM,
) -> LRESULT {
  if msg == winuser::WM_DESTROY {
    winuser::PostQuitMessage(0);
    return 0;
  }

  // click on the icon
  if msg == WM_USER_TRAYICON {
    let mut rect = RECT::default();
    let nid = shellapi::NOTIFYICONIDENTIFIER {
      hWnd: hwnd,
      cbSize: std::mem::size_of::<shellapi::NOTIFYICONIDENTIFIER>() as _,
      uID: WM_USER_TRAYICON_UID,
      ..Default::default()
    };

    shellapi::Shell_NotifyIconGetRect(&nid, &mut rect);

    let dpi = hwnd_dpi(hwnd);
    let scale_factor = dpi_to_scale_factor(dpi);

    let mut cursor = POINT { x: 0, y: 0 };
    winuser::GetCursorPos(&mut cursor as _);
    SYSTEM_TRAY_STASH.with(|stash| {
      if let Some(ref data) = *stash.borrow() {
        match lparam as u32 {
          // Left click tray icon
          winuser::WM_LBUTTONUP => {
            data.sender.send_event(Event::TrayEvent {
              event: TrayEvent::LeftClick,
              position: LogicalPosition::new(cursor.x, cursor.y).to_physical(scale_factor),
              bounds: Rectangle {
                position: LogicalPosition::new(rect.left, rect.top).to_physical(scale_factor),
                size: LogicalSize::new(rect.right - rect.left, rect.bottom - rect.top)
                  .to_physical(scale_factor),
              },
            });
          }

          // Right click tray icon
          winuser::WM_RBUTTONUP => {
            data.sender.send_event(Event::TrayEvent {
              event: TrayEvent::RightClick,
              position: LogicalPosition::new(cursor.x, cursor.y).to_physical(scale_factor),
              bounds: Rectangle {
                position: LogicalPosition::new(rect.left, rect.top).to_physical(scale_factor),
                size: LogicalSize::new(rect.right - rect.left, rect.bottom - rect.top)
                  .to_physical(scale_factor),
              },
            });

            // show menu on right click
            if let Some(menu) = data.system_tray.hmenu {
              show_tray_menu(hwnd, menu, cursor.x, cursor.y);
            }
          }

          // Double click tray icon
          winuser::WM_LBUTTONDBLCLK => {
            data.sender.send_event(Event::TrayEvent {
              event: TrayEvent::DoubleClick,
              position: LogicalPosition::new(cursor.x, cursor.y).to_physical(scale_factor),
              bounds: Rectangle {
                position: LogicalPosition::new(rect.left, rect.top).to_physical(scale_factor),
                size: LogicalSize::new(rect.right - rect.left, rect.bottom - rect.top)
                  .to_physical(scale_factor),
              },
            });
          }

          _ => {}
        }
      }
    });
  }

  return winuser::DefWindowProcW(hwnd, msg, wparam, lparam);
}

impl Drop for WindowsLoopData {
  fn drop(&mut self) {
    self.system_tray.remove();
  }
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
