// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use super::menu::{subclass_proc, to_wstring, Menu, MenuHandler};
use crate::{
  dpi::{PhysicalPosition, PhysicalSize},
  error::OsError as RootOsError,
  event::{Event, Rectangle, TrayEvent},
  event_loop::EventLoopWindowTarget,
  menu::MenuType,
};
use std::cell::RefCell;
use winapi::{
  ctypes::{c_ulong, c_ushort},
  shared::{
    basetsd::DWORD_PTR,
    guiddef::GUID,
    minwindef::{DWORD, HINSTANCE, LPARAM, LRESULT, UINT, WPARAM},
    ntdef::LPCWSTR,
    windef::{HBRUSH, HICON, HMENU, HWND, POINT, RECT},
  },
  um::{
    commctrl::SetWindowSubclass,
    libloaderapi,
    shellapi::{self, NIF_ICON, NIF_MESSAGE, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW},
    winuser::{self, CW_USEDEFAULT, LR_DEFAULTCOLOR, WNDCLASSW, WS_OVERLAPPEDWINDOW},
  },
};

thread_local!(static WININFO_STASH: RefCell<Option<WindowsLoopData>> = RefCell::new(None));

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
  ) -> Result<SystemTray, RootOsError> {
    let mut hmenu: Option<HMENU> = None;
    if let Some(menu) = self.tray_menu {
      hmenu = Some(menu.into_hmenu());
    }

    let class_name = to_wstring("tao_system_tray_app");
    unsafe {
      let _hinstance: HINSTANCE = libloaderapi::GetModuleHandleA(std::ptr::null_mut());
      let wnd = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: 0 as HINSTANCE,
        hIcon: winuser::LoadIconW(0 as HINSTANCE, winuser::IDI_APPLICATION),
        hCursor: winuser::LoadCursorW(0 as HINSTANCE, winuser::IDI_APPLICATION),
        hbrBackground: 16 as HBRUSH,
        lpszMenuName: 0 as LPCWSTR,
        lpszClassName: class_name.as_ptr(),
      };
      if winuser::RegisterClassW(&wnd) == 0 {
        // FIXME: os_error dont seems to work :(
        // os_error!(OsError::CreationError("Error registering window"))
        // return Err(OsError::CreationError("Error registering window"));
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
        0 as HWND,
        0 as HMENU,
        0 as HINSTANCE,
        std::ptr::null_mut(),
      );
      if hwnd == std::ptr::null_mut() {
        //return os_error!(OsError::CreationError("Error creating window"));
      }

      let mut nid = get_nid_struct(&hwnd);
      nid.uID = WM_USER_TRAYICON_UID;
      nid.uFlags = NIF_MESSAGE;
      nid.uCallbackMessage = WM_USER_TRAYICON;
      if shellapi::Shell_NotifyIconW(NIM_ADD, &mut nid as *mut NOTIFYICONDATAW) == 0 {
        //return os_error!(OsError::CreationError("Error registering app icon"));
      }

      let app_system_tray = SystemTray { hwnd, hmenu };
      app_system_tray.set_icon_from_buffer(&self.icon, 32, 32);

      // create the handler
      let event_loop_runner = window_target.p.runner_shared.clone();
      let menu_handler = MenuHandler::new(
        Box::new(move |event| {
          if let Ok(e) = event.map_nonuser_event() {
            event_loop_runner.send_event(e)
          }
        }),
        MenuType::SystemTray,
      );
      // TODO: Remove `WININFO_STASH` thread_local and save hmenu into the box
      WININFO_STASH.with(|stash| {
        let data = WindowsLoopData {
          system_tray: app_system_tray,
          sender: menu_handler,
        };
        (*stash.borrow_mut()) = Some(data);
      });

      let event_loop_runner = window_target.p.runner_shared.clone();
      let menu_handler = MenuHandler::new(
        Box::new(move |event| {
          if let Ok(e) = event.map_nonuser_event() {
            event_loop_runner.send_event(e)
          }
        }),
        MenuType::SystemTray,
      );
      let sender: *mut MenuHandler = Box::into_raw(Box::new(menu_handler));
      SetWindowSubclass(hwnd as _, Some(subclass_proc), 0, sender as DWORD_PTR);

      return Ok(SystemTray { hwnd, hmenu });
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
    unsafe {
      // we should align our pointer to windows directory
      match winuser::LookupIconIdFromDirectoryEx(
        buffer.as_ptr() as *mut _,
        1,
        width as i32,
        height as i32,
        LR_DEFAULTCOLOR,
      ) as isize
      {
        0 => {
          debug!("Unable to LookupIconIdFromDirectoryEx");
          return;
        }
        offset => {
          // once we got the pointer offset for the directory
          // lets create our resource
          match winuser::CreateIconFromResourceEx(
            buffer.as_ptr().offset(offset) as *mut _,
            buffer.len() as u32,
            1,
            0x00030000,
            0,
            0,
            LR_DEFAULTCOLOR,
          ) {
            // windows is really tough on icons
            // if a bad icon is provided it'll fail here or in
            // the LookupIconIdFromDirectoryEx if this is a bad format (example png's)
            // with my tests, even some ICO's were failing...
            hicon if hicon.is_null() => {
              debug!("Unable to CreateIconFromResourceEx");
              return;
            }
            hicon => {
              // finally.... we can set the icon...
              self.set_hicon(hicon);
            }
          }
        }
      }
    }
  }

  // set the icon for our main instance
  fn set_hicon(&self, icon: HICON) {
    unsafe {
      let mut nid = get_nid_struct(&self.hwnd);
      nid.uFlags = NIF_ICON;
      nid.hIcon = icon;
      nid.uID = WM_USER_TRAYICON_UID;
      if shellapi::Shell_NotifyIconW(NIM_MODIFY, &mut nid as *mut NOTIFYICONDATAW) == 0 {
        debug!("Error setting icon");
        return;
      }
    }
  }

  pub fn shutdown(&self) {
    unsafe {
      let mut nid = get_nid_struct(&self.hwnd);
      nid.uFlags = NIF_ICON;
      if shellapi::Shell_NotifyIconW(NIM_DELETE, &mut nid as *mut NOTIFYICONDATAW) == 0 {
        debug!("Error removing icon");
        return;
      }
    }
  }
}
// basic NID for our icon
pub(crate) fn get_nid_struct(hwnd: &HWND) -> NOTIFYICONDATAW {
  NOTIFYICONDATAW {
    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as DWORD,
    hWnd: *hwnd,
    uID: 0x1 as UINT,
    uFlags: 0 as UINT,
    uCallbackMessage: 0 as UINT,
    hIcon: 0 as HICON,
    szTip: [0 as u16; 128],
    dwState: 0 as DWORD,
    dwStateMask: 0 as DWORD,
    szInfo: [0 as u16; 256],
    u: Default::default(),
    szInfoTitle: [0 as u16; 64],
    dwInfoFlags: 0 as UINT,
    guidItem: GUID {
      Data1: 0 as c_ulong,
      Data2: 0 as c_ushort,
      Data3: 0 as c_ushort,
      Data4: [0; 8],
    },
    hBalloonIcon: 0 as HICON,
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
    shellapi::Shell_NotifyIconGetRect(&nid as *const _, &mut rect as *mut _);

    WININFO_STASH.with(|stash| {
      let stash = stash.borrow();
      let stash = stash.as_ref();
      if let Some(stash) = stash {
        match lparam as u32 {
          // Left click tray icon
          winuser::WM_LBUTTONUP => {
            stash.sender.send_event(Event::TrayEvent {
              event: TrayEvent::LeftClick,
              bounds: Rectangle {
                position: PhysicalPosition::new(rect.left as _, rect.top as _),
                size: PhysicalSize::new(
                  (rect.right - rect.left) as _,
                  (rect.bottom - rect.top) as _,
                ),
              },
            });
          }

          // Right click tray icon
          winuser::WM_RBUTTONUP => {
            let mut p = POINT { x: 0, y: 0 };
            winuser::GetCursorPos(&mut p as *mut POINT);

            stash.sender.send_event(Event::TrayEvent {
              event: TrayEvent::RightClick,
              bounds: Rectangle {
                position: PhysicalPosition::new(5.0, 5.0),
                size: PhysicalSize::new(5.0, 5.0),
              },
            });

            // show menu on right click
            if let Some(menu) = stash.system_tray.hmenu {
              // set the popup foreground
              winuser::SetForegroundWindow(hwnd);
              // track the click
              winuser::TrackPopupMenu(
                menu,
                0,
                p.x,
                p.y,
                // align bottom / right, maybe we could expose this later..
                (winuser::TPM_BOTTOMALIGN | winuser::TPM_LEFTALIGN) as i32,
                hwnd,
                std::ptr::null_mut(),
              );
            }
          }

          // Double click tray icon
          winuser::WM_LBUTTONDBLCLK => {
            stash.sender.send_event(Event::TrayEvent {
              event: TrayEvent::DoubleClick,
              bounds: Rectangle {
                position: PhysicalPosition::new(5.0, 5.0),
                size: PhysicalSize::new(5.0, 5.0),
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
    self.system_tray.shutdown();
  }
}
