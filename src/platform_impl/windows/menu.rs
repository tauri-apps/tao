use raw_window_handle::RawWindowHandle;
use std::os::windows::ffi::OsStrExt;
use winapi::{
  shared::{basetsd, minwindef, windef},
  um::{commctrl, winuser},
};

use crate::{
  event::Event,
  menu::{Menu, MenuId, MenuItem, MenuType},
};

pub struct MenuHandler {
  send_event: Box<dyn Fn(Event<'static, ()>)>,
}

#[allow(non_snake_case)]
impl MenuHandler {
  pub fn new(send_event: Box<dyn Fn(Event<'static, ()>)>) -> MenuHandler {
    MenuHandler { send_event }
  }
  fn send_click_event(&self, menu_id: u32) {
    (self.send_event)(Event::MenuEvent {
      menu_id: MenuId(menu_id),
      origin: MenuType::Menubar,
    });
  }
}

pub fn initialize(menu: Vec<Menu>, window_handle: RawWindowHandle, menu_handler: MenuHandler) {
  if let RawWindowHandle::Windows(handle) = window_handle {
    let sender: *mut MenuHandler = Box::into_raw(Box::new(menu_handler));

    unsafe {
      commctrl::SetWindowSubclass(
        handle.hwnd as *mut _,
        Some(subclass_proc),
        0,
        sender as basetsd::DWORD_PTR,
      );

      let app_menu = winuser::CreateMenu();
      let mut main_menu_position = 0;
      for menu in menu {
        let sub_menu = winuser::CreateMenu();
        let mut sub_menu_position = 0;
        for item in &menu.items {
          let sub_item = match item {
            MenuItem::Custom(custom_menu) => {
              make_menu_item(Some(custom_menu._id.0), custom_menu.name)
            }
            // Let's support only custom menu in windows for now
            _ => None,
          };
          if let Some(sub_item) = sub_item {
            sub_menu_position += 1;
            winuser::InsertMenuItemW(sub_menu, sub_menu_position, 0, &sub_item as *const _);
          }
        }

        let item = winuser::MENUITEMINFOW {
          cbSize: std::mem::size_of::<winuser::MENUITEMINFOW>() as u32,
          fMask: winuser::MIIM_STRING | winuser::MIIM_SUBMENU,
          fType: winuser::MFT_STRING,
          fState: winuser::MFS_ENABLED,
          wID: 0,
          hSubMenu: sub_menu,
          hbmpChecked: std::ptr::null_mut(),
          hbmpUnchecked: std::ptr::null_mut(),
          dwItemData: 0,
          dwTypeData: to_wstring(&menu.title).as_mut_ptr(),
          cch: 5,
          hbmpItem: std::ptr::null_mut(),
        };
        main_menu_position += 1;
        winuser::InsertMenuItemW(app_menu, main_menu_position, 0, &item as *const _);
      }

      winuser::SetMenu(handle.hwnd as *mut _, app_menu);
    }
  }
}

fn make_menu_item(id: Option<u32>, title: &str) -> Option<winuser::MENUITEMINFOW> {
  let mut real_id = 0;
  if let Some(id) = id {
    real_id = id;
  }

  Some(winuser::MENUITEMINFOW {
    cbSize: std::mem::size_of::<winuser::MENUITEMINFOW>() as u32,
    fMask: winuser::MIIM_STRING | winuser::MIIM_ID,
    fType: winuser::MFT_STRING,
    fState: winuser::MFS_ENABLED,
    // It represent the unique menu ID
    // that we can get inside our w_param from the subclass_proc
    wID: real_id,
    hSubMenu: std::ptr::null_mut(),
    hbmpChecked: std::ptr::null_mut(),
    hbmpUnchecked: std::ptr::null_mut(),
    dwItemData: 0,
    dwTypeData: to_wstring(title).as_mut_ptr(),
    cch: 5,
    hbmpItem: std::ptr::null_mut(),
  })
}

fn to_wstring(str: &str) -> Vec<u16> {
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
