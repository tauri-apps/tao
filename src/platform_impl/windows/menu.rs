// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use raw_window_handle::RawWindowHandle;
use std::os::windows::ffi::OsStrExt;

use winapi::{
  shared::{basetsd, minwindef, windef},
  um::{commctrl, winuser},
};

use std::ptr::null;

use crate::{
  event::Event,
  menu::{MenuId, MenuType, SystemMenu},
};

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
}

#[derive(Debug, Clone)]
pub struct MenuItem(pub(crate) i32);

impl MenuItem {
  pub fn set_enabled(&mut self, _is_enabled: bool) {}
  pub fn set_title(&mut self, _title: &str) {}
  pub fn set_selected(&mut self, _is_selected: bool) {}
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

impl Menu {
  /// Create a new menu for a window.
  pub fn new() -> Menu {
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

  pub fn add_children(&mut self, menu: Self, title: &str, enabled: bool) {
    unsafe {
      let mut flags = winuser::MF_POPUP;
      if !enabled {
        flags |= winuser::MF_GRAYED;
      }

      winuser::AppendMenuW(
        self.hmenu,
        flags,
        menu.into_hmenu() as basetsd::UINT_PTR,
        to_wstring(title).as_mut_ptr(),
      );
    }
  }

  pub fn add_separator(&mut self) {
    unsafe {
      winuser::AppendMenuW(self.hmenu, winuser::MF_SEPARATOR, 0, null());
    }
  }

  pub fn add_system_item(&mut self, _item: SystemMenu, _menu_type: MenuType) -> Option<MenuItem> {
    None
  }

  pub fn add_custom_item(
    &mut self,
    id: MenuId,
    _menu_type: MenuType,
    text: &str,
    _key: Option<&str>,
    enabled: bool,
    selected: bool,
  ) -> MenuItem {
    unsafe {
      let mut flags = winuser::MF_STRING;
      if !enabled {
        flags |= winuser::MF_GRAYED;
      }
      if selected {
        flags |= winuser::MF_CHECKED;
      }
      println!("ID: {}", id.0);
      let item = winuser::AppendMenuW(
        self.hmenu,
        flags,
        id.0 as basetsd::UINT_PTR,
        to_wstring(&text).as_mut_ptr(),
      );
      println!("item: {}", item);
      MenuItem(item)
    }
  }
}

pub fn initialize(menu_builder: Menu, window_handle: RawWindowHandle, menu_handler: MenuHandler) {
  if let RawWindowHandle::Windows(handle) = window_handle {
    let sender: *mut MenuHandler = Box::into_raw(Box::new(menu_handler));

    unsafe {
      commctrl::SetWindowSubclass(
        handle.hwnd as *mut _,
        Some(subclass_proc),
        0,
        sender as basetsd::DWORD_PTR,
      );
      winuser::SetMenu(handle.hwnd as *mut _, menu_builder.into_hmenu());
    }
  }
}

pub(crate) fn to_wstring(str: &str) -> Vec<u16> {
  let v: Vec<u16> = std::ffi::OsStr::new(str)
    .encode_wide()
    .chain(Some(0).into_iter())
    .collect();
  v
}

unsafe extern "system" fn subclass_proc(
  hwnd: windef::HWND,
  u_msg: minwindef::UINT,
  w_param: minwindef::WPARAM,
  l_param: minwindef::LPARAM,
  _id: basetsd::UINT_PTR,
  data: basetsd::DWORD_PTR,
) -> minwindef::LRESULT {
  match u_msg {
    winuser::WM_COMMAND => {
      let proxy = &mut *(data as *mut MenuHandler);
      proxy.send_click_event(w_param as u32);
      0
    }
    winuser::WM_DESTROY => {
      Box::from_raw(data as *mut MenuHandler);
      0
    }
    _ => commctrl::DefSubclassProc(hwnd, u_msg, w_param, l_param),
  }
}
