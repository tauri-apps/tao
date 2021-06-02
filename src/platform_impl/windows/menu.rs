// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use raw_window_handle::RawWindowHandle;
use std::{ffi::CString, os::windows::ffi::OsStrExt, sync::Mutex};

use winapi::{
  shared::{basetsd, minwindef, windef},
  um::{commctrl, winuser},
};

use crate::{
  event::{Event, WindowEvent},
  menu::{CustomMenuItem as RootCustomMenuItem, MenuId, MenuItem, MenuType},
  platform_impl::platform::WindowId,
  window::WindowId as RootWindowId,
};

const CUT_ID: usize = 5001;
const COPY_ID: usize = 5002;
const PASTE_ID: usize = 5003;
const HIDE_ID: usize = 5004;
const CLOSE_ID: usize = 5005;
const QUIT_ID: usize = 5006;
const MINIMIZE_ID: usize = 5007;

lazy_static! {
  static ref MENU_IDS: Mutex<Vec<usize>> = Mutex::new(vec![]);
}

pub struct MenuHandler {
  menu_type: MenuType,
  send_event: Box<dyn Fn(Event<'static, ()>)>,
}

impl MenuHandler {
  pub fn new(send_event: Box<dyn Fn(Event<'static, ()>)>, menu_type: MenuType) -> MenuHandler {
    MenuHandler {
      send_event,
      menu_type,
    }
  }
  pub fn send_click_event(&self, menu_id: u32) {
    (self.send_event)(Event::MenuEvent {
      menu_id: MenuId(menu_id),
      origin: self.menu_type,
    });
  }

  pub fn send_event(&self, event: Event<'static, ()>) {
    (self.send_event)(event);
  }
}

#[derive(Debug, Clone)]
pub struct CustomMenuItem(pub(crate) u32, windef::HMENU);

impl CustomMenuItem {
  pub fn id(&self) -> MenuId {
    MenuId(self.0)
  }
  pub fn set_enabled(&mut self, enabled: bool) {
    unsafe {
      winuser::EnableMenuItem(
        self.1,
        self.0,
        match enabled {
          true => winuser::MF_ENABLED,
          false => winuser::MF_DISABLED,
        },
      );
    }
  }
  pub fn set_title(&mut self, title: &str) {
    unsafe {
      let mut info = winuser::MENUITEMINFOA {
        cbSize: std::mem::size_of::<winuser::MENUITEMINFOA>() as _,
        fMask: winuser::MIIM_STRING,
        ..Default::default()
      };
      let c_str = CString::new(title).unwrap();
      info.dwTypeData = c_str.as_ptr() as _;

      winuser::SetMenuItemInfoA(self.1, self.0, minwindef::FALSE, &info);
    }
  }
  pub fn set_selected(&mut self, selected: bool) {
    unsafe {
      winuser::CheckMenuItem(
        self.1,
        self.0,
        match selected {
          true => winuser::MF_CHECKED,
          false => winuser::MF_UNCHECKED,
        },
      );
    }
  }

  // todo: set custom icon to the menu item
  pub fn set_icon(&mut self, _icon: Vec<u8>) {}
}

#[derive(Debug, Clone)]
pub struct Menu {
  hmenu: windef::HMENU,
}

impl Drop for Menu {
  fn drop(&mut self) {
    unsafe {
      winuser::DestroyMenu(self.hmenu);
    }
  }
}

unsafe impl Send for Menu {}
unsafe impl Sync for Menu {}

impl Default for Menu {
  fn default() -> Self {
    Menu::new()
  }
}

impl Menu {
  pub fn new() -> Self {
    unsafe {
      let hmenu = winuser::CreateMenu();
      Menu { hmenu }
    }
  }

  pub fn new_popup_menu() -> Menu {
    unsafe {
      let hmenu = winuser::CreatePopupMenu();
      Menu { hmenu }
    }
  }

  pub fn into_hmenu(self) -> windef::HMENU {
    let hmenu = self.hmenu;
    std::mem::forget(self);
    hmenu
  }

  pub fn add_item(&mut self, item: MenuItem, _menu_type: MenuType) -> Option<RootCustomMenuItem> {
    let menu_item = match item {
      MenuItem::Separator => {
        unsafe {
          winuser::AppendMenuW(self.hmenu, winuser::MF_SEPARATOR, 0, std::ptr::null());
        };
        None
      }
      MenuItem::Submenu {
        enabled,
        menu_platform,
        title,
      } => {
        unsafe {
          let mut flags = winuser::MF_POPUP;
          if !enabled {
            flags |= winuser::MF_DISABLED;
          }

          winuser::AppendMenuW(
            self.hmenu,
            flags,
            menu_platform.into_hmenu() as _,
            to_wstring(&title).as_mut_ptr(),
          );
        }

        None
      }
      MenuItem::Custom {
        menu_id,
        enabled,
        selected,
        text,
        keyboard_accelerator: _,
      } => {
        unsafe {
          let mut flags = winuser::MF_STRING;
          if !enabled {
            flags |= winuser::MF_GRAYED;
          }
          if selected {
            flags |= winuser::MF_CHECKED;
          }

          // FIXME: add keyboard accelerators
          winuser::AppendMenuW(
            self.hmenu,
            flags,
            menu_id.0 as _,
            to_wstring(&text).as_mut_ptr(),
          );
          MENU_IDS.lock().unwrap().push(menu_id.0 as _);
          Some(CustomMenuItem(menu_id.0, self.hmenu))
        }
      }

      MenuItem::Cut => {
        unsafe {
          winuser::AppendMenuW(
            self.hmenu,
            winuser::MF_STRING,
            CUT_ID,
            to_wstring("&Cut\tCtrl+X").as_mut_ptr(),
          );
        }
        None
      }
      MenuItem::Copy => {
        unsafe {
          winuser::AppendMenuW(
            self.hmenu,
            winuser::MF_STRING,
            COPY_ID,
            to_wstring("&Copy\tCtrl+C").as_mut_ptr(),
          );
        }
        None
      }
      MenuItem::Paste => {
        unsafe {
          winuser::AppendMenuW(
            self.hmenu,
            winuser::MF_STRING,
            PASTE_ID,
            to_wstring("&Paste\tCtrl+V").as_mut_ptr(),
          );
        }
        None
      }
      MenuItem::Hide => {
        unsafe {
          winuser::AppendMenuW(
            self.hmenu,
            winuser::MF_STRING,
            HIDE_ID,
            to_wstring("&Hide\tCtrl+H").as_mut_ptr(),
          );
        }
        None
      }
      MenuItem::CloseWindow => {
        unsafe {
          winuser::AppendMenuW(
            self.hmenu,
            winuser::MF_STRING,
            CLOSE_ID,
            to_wstring("&Close\tAlt+F4").as_mut_ptr(),
          );
        }
        None
      }
      MenuItem::Quit => {
        unsafe {
          winuser::AppendMenuW(
            self.hmenu,
            winuser::MF_STRING,
            QUIT_ID,
            to_wstring("&Quit").as_mut_ptr(),
          );
        }
        None
      }
      MenuItem::Minimize => {
        unsafe {
          winuser::AppendMenuW(
            self.hmenu,
            winuser::MF_STRING,
            MINIMIZE_ID,
            to_wstring("&Minimize").as_mut_ptr(),
          );
        }
        None
      }
      // FIXME: create all shortcuts of MenuItem if possible...
      // like linux?
      _ => None,
    };
    if let Some(menu_item) = menu_item {
      return Some(RootCustomMenuItem(CustomMenuItem(menu_item.0, self.hmenu)));
    }
    None
  }
}

pub fn initialize(
  menu_builder: Menu,
  window_handle: RawWindowHandle,
  menu_handler: MenuHandler,
) -> Option<windef::HMENU> {
  if let RawWindowHandle::Windows(handle) = window_handle {
    let sender: *mut MenuHandler = Box::into_raw(Box::new(menu_handler));
    let menu = menu_builder.into_hmenu();

    unsafe {
      commctrl::SetWindowSubclass(handle.hwnd as _, Some(subclass_proc), 0, sender as _);
      winuser::SetMenu(handle.hwnd as _, menu);
    }
    Some(menu)
  } else {
    None
  }
}

pub(crate) fn to_wstring(str: &str) -> Vec<u16> {
  std::ffi::OsStr::new(str)
    .encode_wide()
    .chain(Some(0).into_iter())
    .collect()
}

pub(crate) unsafe extern "system" fn subclass_proc(
  hwnd: windef::HWND,
  msg: minwindef::UINT,
  wparam: minwindef::WPARAM,
  lparam: minwindef::LPARAM,
  _id: basetsd::UINT_PTR,
  data: basetsd::DWORD_PTR,
) -> minwindef::LRESULT {
  let proxy = &mut *(data as *mut MenuHandler);
  match msg {
    winuser::WM_COMMAND => {
      match wparam {
        CUT_ID => {
          execute_edit_command(EditCommands::Cut);
        }
        COPY_ID => {
          execute_edit_command(EditCommands::Copy);
        }
        PASTE_ID => {
          execute_edit_command(EditCommands::Paste);
        }
        HIDE_ID => {
          winuser::ShowWindow(hwnd, winuser::SW_HIDE);
        }
        CLOSE_ID => {
          proxy.send_event(Event::WindowEvent {
            window_id: RootWindowId(WindowId(hwnd)),
            event: WindowEvent::CloseRequested,
          });
        }
        QUIT_ID => {
          proxy.send_event(Event::LoopDestroyed);
        }
        MINIMIZE_ID => {
          winuser::ShowWindow(hwnd, winuser::SW_MINIMIZE);
        }
        _ => {
          if MENU_IDS.lock().unwrap().contains(&wparam) {
            proxy.send_click_event(wparam as u32);
          }
        }
      }
      0
    }
    winuser::WM_DESTROY => {
      Box::from_raw(data as *mut MenuHandler);
      0
    }
    _ => commctrl::DefSubclassProc(hwnd, msg, wparam, lparam),
  }
}

enum EditCommands {
  Copy,
  Cut,
  Paste,
}
fn execute_edit_command(command: EditCommands) {
  let key = match command {
    EditCommands::Copy => 0x43,  // c
    EditCommands::Cut => 0x58,   // x
    EditCommands::Paste => 0x56, // v
  };

  unsafe {
    let mut inputs: [winuser::INPUT; 4] = std::mem::zeroed();
    inputs[0].type_ = winuser::INPUT_KEYBOARD;
    inputs[0].u.ki_mut().wVk = winuser::VK_CONTROL as _;

    inputs[1].type_ = winuser::INPUT_KEYBOARD;
    inputs[1].u.ki_mut().wVk = key;

    inputs[2].type_ = winuser::INPUT_KEYBOARD;
    inputs[2].u.ki_mut().wVk = key;
    inputs[2].u.ki_mut().dwFlags = winuser::KEYEVENTF_KEYUP;

    inputs[3].type_ = winuser::INPUT_KEYBOARD;
    inputs[3].u.ki_mut().wVk = winuser::VK_CONTROL as _;
    inputs[3].u.ki_mut().dwFlags = winuser::KEYEVENTF_KEYUP;

    winuser::SendInput(
      inputs.len() as _,
      inputs.as_mut_ptr(),
      std::mem::size_of::<winuser::INPUT>() as _,
    );
  }
}
