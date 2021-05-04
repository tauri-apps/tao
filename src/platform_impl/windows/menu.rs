use raw_window_handle::RawWindowHandle;
use std::os::windows::ffi::OsStrExt;
use winapi::{
  ctypes::c_void,
  shared::{
    basetsd,
    guiddef::REFIID,
    minwindef,
    minwindef::{DWORD, UINT, ULONG},
    windef,
    windef::{HWND, POINTL},
    winerror::S_OK,
  },
  um::{
    commctrl,
    objidl::IDataObject,
    oleidl::{IDropTarget, IDropTargetVtbl, DROPEFFECT_COPY, DROPEFFECT_NONE},
    shellapi, unknwnbase,
    winnt::HRESULT,
    winuser,
  },
};

use std::{
  cell::RefCell,
  collections::HashMap,
  ptr,
  sync::atomic::{AtomicUsize, Ordering},
};

use crate::menu::{Menu, MenuItem};
use crate::{event::Event, window::WindowId as SuperWindowId};

pub struct MenuHandler {
  window: HWND,
  send_event: Box<dyn Fn(Event<'static, ()>)>,
}

thread_local! {
  static MENU_INDEX: RefCell<u32> = RefCell::new(1);
  static MENU_MAP: RefCell<HashMap<u32, String>> = RefCell::new(HashMap::new());
}

#[allow(non_snake_case)]
impl MenuHandler {
  pub fn new(window: HWND, send_event: Box<dyn Fn(Event<'static, ()>)>) -> MenuHandler {
    MenuHandler { window, send_event }
  }
  fn send_click_event(&self, menu_id: u32) {
    MENU_MAP.with(|cell| {
      let current_hash_map = cell.borrow();
      if let Some(real_menu_id) = current_hash_map.get(&menu_id) {
        (self.send_event)(Event::MenuEvent(real_menu_id.to_string()));
      }
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
              // it's a custom menu, let's add it to our hashmap
              let mut current_id = 0;
              MENU_INDEX.with(|cell| {
                current_id = cell.replace_with(|&mut i| i + 1);
              });

              // save our reference to match later in the click event
              MENU_MAP.with(|cell| {
                cell.borrow_mut().insert(current_id, custom_menu.id.clone());
              });

              make_menu_item(Some(current_id.clone()), custom_menu.name.as_str())
            }
            MenuItem::Separator => None,
            MenuItem::About(_app_name) => make_menu_item(None, "About"),
            MenuItem::CloseWindow => make_menu_item(None, "Close"),
            MenuItem::Quit => make_menu_item(None, "Quit"),
            MenuItem::Hide => make_menu_item(None, "Hide"),
            MenuItem::HideOthers => make_menu_item(None, "Hide Others"),
            MenuItem::ShowAll => make_menu_item(None, "Show All"),
            MenuItem::EnterFullScreen => make_menu_item(None, "Enter Full Screen"),
            MenuItem::Minimize => make_menu_item(None, "Minimize"),
            MenuItem::Zoom => make_menu_item(None, "Zoom"),
            MenuItem::Copy => make_menu_item(None, "Copy"),
            MenuItem::Cut => make_menu_item(None, "Cut"),
            MenuItem::Paste => make_menu_item(None, "Paste"),
            MenuItem::Undo => make_menu_item(None, "Undo"),
            MenuItem::Redo => make_menu_item(None, "Redo"),
            MenuItem::SelectAll => make_menu_item(None, "Select All"),
            MenuItem::Services => None,
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
    // Received on low-word of wParam when WM_COMMAND
    // It represent the unique menu ID
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
      let lo_word = minwindef::LOWORD(w_param as u32);
      proxy.send_click_event(lo_word.into());
      0
    }
    winuser::WM_DESTROY => {
      Box::from_raw(data as *mut MenuHandler);
      0
    }
    _ => commctrl::DefSubclassProc(hwnd, u_msg, w_param, l_param),
  }
}
