use super::menu::{make_menu_item, to_wstring, MenuHandler};
use crate::{
  error::OsError,
  menu::{MenuItem, MenuType},
  platform_impl::EventLoopWindowTarget,
  status_bar::Statusbar as RootStatusbar,
};
use std::cell::RefCell;
use winapi::{
  ctypes::{c_ulong, c_ushort},
  shared::{
    basetsd::ULONG_PTR,
    guiddef::GUID,
    minwindef::{DWORD, HINSTANCE, LPARAM, LRESULT, UINT, WPARAM},
    ntdef::LPCWSTR,
    windef::{HBRUSH, HICON, HMENU, HWND, POINT},
  },
  um::{
    libloaderapi,
    shellapi::{self, NIF_ICON, NIF_MESSAGE, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW},
    winuser::{
      self, CW_USEDEFAULT, LR_DEFAULTCOLOR, MENUINFO, MENUITEMINFOW, MIM_APPLYTOSUBMENUS,
      MIM_STYLE, MNS_NOTIFYBYPOS, WM_USER, WNDCLASSW, WS_OVERLAPPEDWINDOW,
    },
  },
};

pub struct Statusbar {
  hwnd: HWND,
  hmenu: HMENU,
}

thread_local!(static WININFO_STASH: RefCell<Option<WindowsLoopData>> = RefCell::new(None));

struct WindowsLoopData {
  status_bar: Statusbar,
  handler: MenuHandler,
}

impl Statusbar {
  pub fn initialize<T>(
    window_target: &EventLoopWindowTarget<T>,
    status_bar: &RootStatusbar,
  ) -> Result<(), OsError> {
    // create the handler
    let event_loop_runner = window_target.runner_shared.clone();
    let menu_handler = MenuHandler::new(Box::new(move |event| {
      if let Ok(e) = event.map_nonuser_event() {
        unsafe { event_loop_runner.send_event(e) }
      }
    }));
    let class_name = to_wstring("tao_status_bar_app");
    unsafe {
      let _hinstance: HINSTANCE = libloaderapi::GetModuleHandleA(std::ptr::null_mut());
      let wnd = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(subclass_proc),
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
        debug!("Error registering window");
        return Ok(());
      }

      let hwnd = winuser::CreateWindowExW(
        0,
        class_name.as_ptr(),
        to_wstring("tao_status_bar_window").as_ptr(),
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
        debug!("Error creating window");
        return Ok(());
      }

      let mut nid = get_nid_struct(&hwnd);
      nid.uID = 0x1;
      nid.uFlags = NIF_MESSAGE;
      nid.uCallbackMessage = WM_USER + 1;
      if shellapi::Shell_NotifyIconW(NIM_ADD, &mut nid as *mut NOTIFYICONDATAW) == 0 {
        debug!("Error registering app icon");
        return Ok(());
      }

      let hmenu = winuser::CreatePopupMenu();
      let app_statusbar = Statusbar { hwnd, hmenu };
      let icon = std::fs::read(&status_bar.icon).map_err(|e| OsError::new(102, "status bar icon", e))?;
      app_statusbar.set_icon_from_buffer(&icon, 32, 32);

      WININFO_STASH.with(|stash| {
        let data = WindowsLoopData {
          status_bar: app_statusbar,
          handler: menu_handler,
        };
        (*stash.borrow_mut()) = Some(data);
      });

      // Setup menu
      let m = MENUINFO {
        cbSize: std::mem::size_of::<MENUINFO>() as DWORD,
        fMask: MIM_APPLYTOSUBMENUS | MIM_STYLE,
        dwStyle: MNS_NOTIFYBYPOS,
        cyMax: 0 as UINT,
        hbrBack: 0 as HBRUSH,
        dwContextHelpID: 0 as DWORD,
        dwMenuData: 0 as ULONG_PTR,
      };

      if winuser::SetMenuInfo(hmenu, &m as *const MENUINFO) == 0 {
        debug!("Error setting up menu");
        return Ok(());
      }

      for menu_item in &status_bar.items {
        let sub_item = match menu_item {
          // we support only custom menu on windows for now
          MenuItem::Custom(custom_menu) => make_menu_item(Some(custom_menu.id.0), custom_menu.name),
          _ => None,
        };

        if let Some(item) = sub_item {
          // add the item to our HMENU
          if winuser::InsertMenuItemW(hmenu, item.wID, 1, &item as *const MENUITEMINFOW) == 0 {
            debug!("Error adding menu item");
            return Ok(());
          }
        }
      }
    }

    Ok(())
  }

  pub fn set_icon_from_buffer(&self, buffer: &[u8], width: u32, height: u32) {
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
              self.set_icon(hicon);
            }
          }
        }
      }
    }
  }

  // set the icon for our main instance
  fn set_icon(&self, icon: HICON) {
    unsafe {
      let mut nid = get_nid_struct(&self.hwnd);
      nid.uFlags = NIF_ICON;
      nid.hIcon = icon;
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

unsafe extern "system" fn subclass_proc(
  h_wnd: HWND,
  msg: UINT,
  w_param: WPARAM,
  l_param: LPARAM,
) -> LRESULT {
  if msg == winuser::WM_MENUCOMMAND {
    WININFO_STASH.with(|stash| {
      let stash = stash.borrow();
      let stash = stash.as_ref();
      if let Some(stash) = stash {
        let menu_id = winuser::GetMenuItemID(stash.status_bar.hmenu, w_param as i32) as u32;
        stash.handler.send_click_event(menu_id, MenuType::Statusbar);
      }
    });
  }

  if msg == winuser::WM_DESTROY {
    winuser::PostQuitMessage(0);
  }

  // track the click
  if msg == WM_USER + 1 {
    if l_param as UINT == winuser::WM_LBUTTONUP || l_param as UINT == winuser::WM_RBUTTONUP {
      let mut p = POINT { x: 0, y: 0 };
      if winuser::GetCursorPos(&mut p as *mut POINT) == 0 {
        return 1;
      }
      // set the popup foreground
      winuser::SetForegroundWindow(h_wnd);
      WININFO_STASH.with(|stash| {
        let stash = stash.borrow();
        let stash = stash.as_ref();
        if let Some(stash) = stash {
          // track the click
          winuser::TrackPopupMenu(
            stash.status_bar.hmenu,
            0,
            p.x,
            p.y,
            // align bottom / right, maybe we could expose this later..
            (winuser::TPM_BOTTOMALIGN | winuser::TPM_LEFTALIGN) as i32,
            h_wnd,
            std::ptr::null_mut(),
          );
        }
      });
    }
  }

  return winuser::DefWindowProcW(h_wnd, msg, w_param, l_param);
}

impl Drop for WindowsLoopData {
  fn drop(&mut self) {
    self.status_bar.shutdown();
  }
}
