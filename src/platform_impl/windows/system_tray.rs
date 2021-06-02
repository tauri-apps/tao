// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use super::{
  dpi::{dpi_to_scale_factor, hwnd_dpi},
  menu::{subclass_proc, to_wstring, Menu, MenuHandler},
  util,
};
use crate::{
  dpi::{LogicalPosition, LogicalSize},
  error::OsError as RootOsError,
  event::{ClickType, Event, Rectangle},
  event_loop::EventLoopWindowTarget,
  menu::MenuType,
  system_tray::SystemTray as RootSystemTray,
};
use winapi::{
  shared::{
    basetsd::LONG_PTR,
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
        // FIXME: os_error dont seems to work :(
        // os_error!(OsError::CreationError("Error registering window"))
        // return Err(OsError::CreationError("Error registering window"));
      }

      // system tray handler
      let event_loop_runner = window_target.p.runner_shared.clone();
      let menu_handler = MenuHandler::new(
        Box::new(move |event| {
          if let Ok(e) = event.map_nonuser_event() {
            event_loop_runner.send_event(e)
          }
        }),
        MenuType::ContextMenu,
      );
      let app_system_tray = SystemTray {
        // dummy hwnd, will populate it later
        hwnd: std::ptr::null::<HWND>() as _,
        hmenu,
      };
      let data = Box::into_raw(Box::new(WindowsLoopData {
        system_tray: app_system_tray,
        sender: menu_handler,
      }));

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
        data as _,
      );
      if hwnd == std::ptr::null_mut() {
        //return os_error!(OsError::CreationError("Error creating window"));
      }

      let mut nid = NOTIFYICONDATAW {
        uFlags: NIF_MESSAGE,
        hWnd: hwnd,
        uID: WM_USER_TRAYICON_UID,
        uCallbackMessage: WM_USER_TRAYICON,
        ..Default::default()
      };
      if shellapi::Shell_NotifyIconW(NIM_ADD, &mut nid as _) == 0 {
        //return os_error!(OsError::CreationError("Error registering app icon"));
      }

      let app_system_tray = SystemTray { hwnd, hmenu };
      app_system_tray.set_icon_from_buffer(&self.icon, 32, 32);

      // create the handler for tray menu events
      let event_loop_runner = window_target.p.runner_shared.clone();
      let menu_handler = MenuHandler::new(
        Box::new(move |event| {
          if let Ok(e) = event.map_nonuser_event() {
            event_loop_runner.send_event(e)
          }
        }),
        MenuType::ContextMenu,
      );
      let sender: *mut MenuHandler = Box::into_raw(Box::new(menu_handler));
      SetWindowSubclass(hwnd as _, Some(subclass_proc), 0, sender as _);

      return Ok(RootSystemTray(SystemTray { hwnd, hmenu }));
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
}

unsafe extern "system" fn window_proc(
  hwnd: HWND,
  msg: UINT,
  wparam: WPARAM,
  lparam: LPARAM,
) -> LRESULT {
  let mut userdata = winuser::GetWindowLongPtrW(hwnd, winuser::GWL_USERDATA);
  if userdata == 0 && msg == winuser::WM_NCCREATE {
    let createstruct = &*(lparam as *const winuser::CREATESTRUCTW);
    userdata = createstruct.lpCreateParams as LONG_PTR;
    (*(userdata as *mut WindowsLoopData)).system_tray.hwnd = hwnd;
    winuser::SetWindowLongPtrW(hwnd, winuser::GWL_USERDATA, userdata);
  }
  let userdata_ptr = userdata as *mut WindowsLoopData;

  if msg == winuser::WM_DESTROY {
    winuser::PostQuitMessage(0);
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
    if shellapi::Shell_NotifyIconGetRect(&nid as _, &mut rect as _) == 0 {
      // FIXME: os_error dont seems to work :(
      // os_error!(OsError::CreationError("Error registering window"))
      // return Err(OsError::CreationError("Error registering window"))
    };

    let dpi = hwnd_dpi(hwnd);
    let scale_factor = dpi_to_scale_factor(dpi);

    let mut cursor = POINT { x: 0, y: 0 };
    winuser::GetCursorPos(&mut cursor as _);

    match lparam as u32 {
      // Left click tray icon
      winuser::WM_LBUTTONUP => {
        (*userdata_ptr).sender.send_event(Event::TrayEvent {
          event: ClickType::LeftClick,
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
        (*userdata_ptr).sender.send_event(Event::TrayEvent {
          event: ClickType::RightClick,
          position: LogicalPosition::new(cursor.x, cursor.y).to_physical(scale_factor),
          bounds: Rectangle {
            position: LogicalPosition::new(rect.left, rect.top).to_physical(scale_factor),
            size: LogicalSize::new(rect.right - rect.left, rect.bottom - rect.top)
              .to_physical(scale_factor),
          },
        });

        // show menu on right click
        if let Some(menu) = (*userdata_ptr).system_tray.hmenu {
          show_tray_menu(hwnd, menu, cursor.x, cursor.y);
        }
      }

      // Double click tray icon
      winuser::WM_LBUTTONDBLCLK => {
        (*userdata_ptr).sender.send_event(Event::TrayEvent {
          event: ClickType::DoubleClick,
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
