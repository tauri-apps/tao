// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use parking_lot::{Mutex, MutexGuard};
use windows::Win32::{
  Foundation::{HWND, LPARAM, LRESULT, PSTR, WPARAM},
  UI::{
    Input::KeyboardAndMouse::*,
    Shell::*,
    WindowsAndMessaging::{self as win32wm, *},
  },
};

use crate::{
  accelerator::Accelerator,
  event::{Event, WindowEvent},
  keyboard::{KeyCode, ModifiersState},
  menu::Menu as RootMenu,
  menu::{MenuId, MenuItemType, MenusData, NativeMenuItem, MENUS_DATA, MENUS_EVENT_SENDER},
  window::WindowId as RootWindowId,
};

use super::{keyboard::key_to_vk, util, WindowId};

pub struct Menu {
  title: String,
  enabled: bool,
  items: Vec<(MenuItemType, MenuId)>,
  hmenu: HMENU,
}

impl Menu {
  pub fn new(title: &str) -> MenuId {
    let mut menus_data = MENUS_DATA.lock();
    let id = uuid::Uuid::new_v4().to_u128_le() as u16;
    menus_data.menus.insert(
      id,
      Menu {
        enabled: true,
        title: title.into(),
        items: Vec::new(),
        hmenu: unsafe { CreateMenu() },
      },
    );
    id
  }

  pub fn add_custom_item(menu_id: MenuId, item_id: MenuId) {
    let mut menus_data = MENUS_DATA.lock();

    let mut menu_hmenu = HMENU::default();
    {
      if let Some(menu) = menus_data.menus.get_mut(&menu_id) {
        menu.items.push((MenuItemType::Custom, item_id));
        menu_hmenu = menu.hmenu;
      }
    }

    if let Some(item) = menus_data.custom_menu_items.get_mut(&item_id) {
      item.parent_menus.push(menu_id);
      item.add_to_hmenu(menu_hmenu);
    }
  }

  pub fn add_native_item(menu_id: MenuId, item: NativeMenuItem) {
    let mut menus_data = MENUS_DATA.lock();

    let mut menu_hmenu = HMENU::default();
    {
      if let Some(menu) = menus_data.menus.get_mut(&menu_id) {
        menu.items.push((MenuItemType::NativeItem, item.id()));
        menu_hmenu = menu.hmenu;
      }
    }

    item.add_to_hmenu(menu_hmenu);
  }

  pub fn add_submenu(menu_id: MenuId, submenu_id: MenuId) {
    let mut menus_data = MENUS_DATA.lock();

    let mut menu_hmenu = HMENU::default();
    {
      if let Some(menu) = menus_data.menus.get_mut(&menu_id) {
        menu.items.push((MenuItemType::Submenu, submenu_id));
        menu_hmenu = menu.hmenu;
      }
    }

    if let Some(submenu) = menus_data.menus.get(&submenu_id) {
      submenu.add_to_hmenu(menu_hmenu);
    }
  }

  pub(super) fn add_to_hmenu(&self, hmenu: HMENU) {
    let mut flags = MF_POPUP;
    if !self.enabled {
      flags |= MF_DISABLED;
    }
    unsafe {
      AppendMenuW(hmenu, flags, self.hmenu.0 as _, self.title.clone());
    }
  }

  pub(super) fn add_items_to_hmenu(&self, hmenu: HMENU, menus_data: &MutexGuard<'_, MenusData>) {
    for (mtype, id) in self.items.clone() {
      match mtype {
        MenuItemType::Custom => {
          if let Some(item) = menus_data.custom_menu_items.get(&id) {
            item.add_to_hmenu(hmenu)
          }
        }
        MenuItemType::Submenu => {
          if let Some(submenu) = menus_data.menus.get(&id) {
            submenu.add_to_hmenu(hmenu);
          }
        }
        MenuItemType::NativeItem => {
          let item = NativeMenuItem::from_id(id);
          item.add_to_hmenu(hmenu);
        }
      }
    }
  }
}

// FIXME(amrbashir): currently changing `CustomMenuItem` using `set_title`, `set_enabled`, `set_selected`
// doesn't reflect the change in the root menu bar (the horizontal menu bar on top of the window) but does
// reflect the change in submenus.
//
// If the window has the following menu structured attached:
//
// MENUBAR {
//  File -> SUBMENU {
//    Perform Action -> CUSTOMMENUITEM
//  }
//  Edit -> SUBMENU {
//    Perform Action -> CUSTOMMENUITEM
//  }
//  Perform Action -> CUSTOMMENUITEM
// }
//
// then `set_title` is called on the `Perform Action` custom menu item to change its title to `Action Performed`, the menu will be like this:
//
// MENUBAR {
//  File -> SUBMENU {
//    Action Performed -> CUSTOMMENUITEM
//  }
//  Edit -> SUBMENU {
//    Action Performed -> CUSTOMMENUITEM
//  }
//  Perform Action -> CUSTOMMENUITEM
// }
pub struct CustomMenuItem {
  id: u16,
  title: String,
  enabled: bool,
  selected: bool,
  accelerator: Option<Accelerator>,
  parent_menus: Vec<MenuId>,
}

impl CustomMenuItem {
  pub fn new(title: &str, enabled: bool, selected: bool, accel: Option<Accelerator>) -> MenuId {
    let mut menus_data = MENUS_DATA.lock();

    let id = uuid::Uuid::new_v4().to_u128_le() as u16;
    menus_data.custom_menu_items.insert(
      id,
      Self {
        id,
        title: title.into(),
        enabled,
        selected,
        accelerator: accel,
        parent_menus: Vec::new(),
      },
    );

    id
  }

  pub fn set_title(item_id: MenuId, title: &str) {
    let mut menus_data = MENUS_DATA.lock();

    {
      if let Some(item) = menus_data.custom_menu_items.get_mut(&item_id) {
        item.title = title.into();
      }
    }

    if let Some(item) = menus_data.custom_menu_items.get(&item_id) {
      let mut title = title.to_string();
      if let Some(accelerator) = &item.accelerator {
        title.push('\t');
        title.push_str(accelerator.to_str().as_str());
      }
      // NOTE(amrbashir): The title must be a null-terminated string. Otherwise, it will display some gibberish characters at the end.
      title.push_str("\0");
      let info = MENUITEMINFOA {
        cbSize: std::mem::size_of::<MENUITEMINFOA>() as _,
        fMask: MIIM_STRING,
        dwTypeData: PSTR(title.as_ptr() as _),
        ..Default::default()
      };
      for menu_id in &item.parent_menus {
        if let Some(menu) = menus_data.menus.get(menu_id) {
          unsafe {
            SetMenuItemInfoA(menu.hmenu, item.id as _, false, &info);
          }
        }
      }
    }
  }

  pub fn set_enabled(item_id: MenuId, enabled: bool) {
    let mut menus_data = MENUS_DATA.lock();

    {
      if let Some(item) = menus_data.custom_menu_items.get_mut(&item_id) {
        item.enabled = enabled;
      }
    }

    if let Some(item) = menus_data.custom_menu_items.get(&item_id) {
      for menu_id in &item.parent_menus {
        if let Some(menu) = menus_data.menus.get(menu_id) {
          unsafe {
            EnableMenuItem(
              menu.hmenu,
              item.id as _,
              match enabled {
                true => MF_ENABLED,
                false => MF_DISABLED,
              },
            );
          }
        }
      }
    }
  }

  pub fn set_selected(item_id: MenuId, selected: bool) {
    let mut menus_data = MENUS_DATA.lock();

    {
      if let Some(item) = menus_data.custom_menu_items.get_mut(&item_id) {
        item.selected = selected;
      }
    }

    if let Some(item) = menus_data.custom_menu_items.get(&item_id) {
      for menu_id in &item.parent_menus {
        if let Some(menu) = menus_data.menus.get(menu_id) {
          unsafe {
            CheckMenuItem(
              menu.hmenu,
              item.id as _,
              match selected {
                true => MF_CHECKED,
                false => MF_UNCHECKED,
              },
            );
          }
        }
      }
    }
  }

  fn add_to_hmenu(&self, menu: HMENU) {
    let mut flags = MF_STRING;
    if !self.enabled {
      flags |= MF_GRAYED;
    }
    if self.selected {
      flags |= MF_CHECKED;
    }

    let mut title = self.title.clone();
    if let Some(accelerator) = &self.accelerator {
      title.push('\t');
      title.push_str(accelerator.to_str().as_str());
      if let Some(accel) = accelerator.to_menu_accel(self.id) {
        let mut accel_table = MENUS_ACCEL_TABLE.lock();
        accel_table.push(accel);
      }
    }
    unsafe {
      AppendMenuW(menu, flags, self.id as _, title);
    }
  }
}

impl NativeMenuItem {
  fn add_to_hmenu(&self, hmenu: HMENU) {
    match self {
      NativeMenuItem::Separator => unsafe {
        AppendMenuW(hmenu, MF_SEPARATOR, 0, "");
      },
      NativeMenuItem::Copy => unsafe {
        AppendMenuW(hmenu, MF_STRING, self.id() as _, "&Copy\tCtrl+C");
      },
      NativeMenuItem::Cut => unsafe {
        AppendMenuW(hmenu, MF_STRING, self.id() as _, "&Cut\tCtrl+X");
      },
      NativeMenuItem::Paste => unsafe {
        AppendMenuW(hmenu, MF_STRING, self.id() as _, "&Paste\tCtrl+V");
      },
      NativeMenuItem::SelectAll => unsafe {
        AppendMenuW(hmenu, MF_STRING, self.id() as _, "&Select all\tCtrl+A");
      },
      NativeMenuItem::Minimize => unsafe {
        AppendMenuW(hmenu, MF_STRING, self.id() as _, "&Minimize");
      },
      NativeMenuItem::Hide => unsafe {
        AppendMenuW(hmenu, MF_STRING, self.id() as _, "&Hide");
      },
      NativeMenuItem::CloseWindow => unsafe {
        AppendMenuW(hmenu, MF_STRING, self.id() as _, "&Close\tAlt+F4");
      },
      _ => {}
    }
  }
}

const MENU_SUBCLASS_ID: usize = 4568;
pub fn set_for_window(menu: RootMenu, window: HWND) -> HMENU {
  let menu_bar = unsafe { CreateMenu() };

  let menus_data = MENUS_DATA.lock();

  if let Some(menu) = menus_data.menus.get(&menu.id()) {
    menu.add_items_to_hmenu(menu_bar, &menus_data);
  }

  unsafe {
    SetWindowSubclass(window, Some(subclass_proc), MENU_SUBCLASS_ID, 0);
    SetMenu(window, menu_bar);
  }

  menu_bar
}

pub fn unset_for_window(window: HWND) {
  unsafe {
    RemoveWindowSubclass(window, Some(subclass_proc), MENU_SUBCLASS_ID);
    SetMenu(window, HMENU::default());
  }
}

pub unsafe extern "system" fn subclass_proc(
  hwnd: HWND,
  msg: u32,
  wparam: WPARAM,
  lparam: LPARAM,
  _id: usize,
  _subclass_input_ptr: usize,
) -> LRESULT {
  match msg {
    win32wm::WM_COMMAND => {
      let menu_id = util::LOWORD(wparam.0 as u32);
      match menu_id {
        _ if menu_id == NativeMenuItem::id(&NativeMenuItem::Copy) => {
          execute_edit_command(EditCommand::Copy);
        }
        _ if menu_id == NativeMenuItem::id(&NativeMenuItem::Cut) => {
          execute_edit_command(EditCommand::Cut);
        }
        _ if menu_id == NativeMenuItem::id(&NativeMenuItem::Paste) => {
          execute_edit_command(EditCommand::Paste);
        }
        _ if menu_id == NativeMenuItem::id(&NativeMenuItem::SelectAll) => {
          execute_edit_command(EditCommand::SelectAll);
        }
        _ if menu_id == NativeMenuItem::id(&NativeMenuItem::Minimize) => {
          ShowWindow(hwnd, SW_MINIMIZE);
        }
        _ if menu_id == NativeMenuItem::id(&NativeMenuItem::Hide) => {
          ShowWindow(hwnd, SW_HIDE);
        }
        _ if menu_id == NativeMenuItem::id(&NativeMenuItem::CloseWindow) => {
          MENUS_EVENT_SENDER.with(|menus_event_sender| {
            if let Some(sender) = &*menus_event_sender.borrow() {
              sender.send_event(Event::WindowEvent {
                window_id: RootWindowId(WindowId(hwnd.0)),
                event: WindowEvent::CloseRequested,
              });
            }
          });
        }
        _ => {
          let mut is_a_menu_event = false;
          {
            let menus_data = MENUS_DATA.lock();

            if menus_data.custom_menu_items.get(&menu_id).is_some() {
              is_a_menu_event = true;
            }
          }
          if is_a_menu_event {
            MENUS_EVENT_SENDER.with(|menus_event_sender| {
              if let Some(sender) = &*menus_event_sender.borrow() {
                sender.send_menu_event(menu_id, Some(RootWindowId(WindowId(hwnd.0))));
              }
            });
          }
        }
      }
      LRESULT(0)
    }
    _ => DefSubclassProc(hwnd, msg, wparam, lparam),
  }
}

enum EditCommand {
  Copy,
  Cut,
  Paste,
  SelectAll,
}
fn execute_edit_command(command: EditCommand) {
  let key = match command {
    EditCommand::Copy => 0x43,      // c
    EditCommand::Cut => 0x58,       // x
    EditCommand::Paste => 0x56,     // v
    EditCommand::SelectAll => 0x41, // a
  };

  unsafe {
    let mut inputs: [INPUT; 4] = std::mem::zeroed();
    inputs[0].r#type = INPUT_KEYBOARD;
    inputs[0].Anonymous.ki.wVk = VK_CONTROL as _;

    inputs[1].r#type = INPUT_KEYBOARD;
    inputs[1].Anonymous.ki.wVk = key as VIRTUAL_KEY;

    inputs[2].r#type = INPUT_KEYBOARD;
    inputs[2].Anonymous.ki.wVk = key as VIRTUAL_KEY;
    inputs[2].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

    inputs[3].r#type = INPUT_KEYBOARD;
    inputs[3].Anonymous.ki.wVk = VK_CONTROL as _;
    inputs[3].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

    SendInput(
      inputs.len() as _,
      inputs.as_mut_ptr(),
      std::mem::size_of::<INPUT>() as _,
    );
  }
}

lazy_static! {
  static ref MENUS_ACCEL_TABLE: Mutex<Vec<ACCEL>> = Mutex::new(Vec::new());
}

pub(super) fn get_haccel() -> HACCEL {
  let accel_table = MENUS_ACCEL_TABLE.lock();

  unsafe {
    win32wm::CreateAcceleratorTableW(
      accel_table.as_slice() as *const _ as *const _,
      accel_table.len() as _,
    )
  }
}
impl Accelerator {
  /// Converts [`Accelerator`] to [`ACCEL`].
  fn to_menu_accel(&self, id: MenuId) -> Option<ACCEL> {
    let mut virt_key = FVIRTKEY;
    let key_mods: ModifiersState = self.mods;
    if key_mods.control_key() {
      virt_key |= FCONTROL;
    }
    if key_mods.alt_key() {
      virt_key |= FALT;
    }
    if key_mods.shift_key() {
      virt_key |= FSHIFT;
    }

    let raw_key = if let Some(vk_code) = key_to_vk(&self.key) {
      let mod_code = vk_code >> 8;
      if mod_code & 0x1 != 0 {
        virt_key |= FSHIFT;
      }
      if mod_code & 0x02 != 0 {
        virt_key |= FCONTROL;
      }
      if mod_code & 0x04 != 0 {
        virt_key |= FALT;
      }
      vk_code & 0x00ff
    } else {
      dbg!("Failed to convert key {:?} into virtual key code", self.key);
      return None;
    };

    Some(ACCEL {
      fVirt: virt_key as u8,
      key: raw_key as u16,
      cmd: id,
    })
  }
  /// Formats [`Accelerator`] as a Windows hotkey string.
  fn to_str(&self) -> String {
    let mut s = String::new();
    let key_mods: ModifiersState = self.mods;
    if key_mods.control_key() {
      s.push_str("Ctrl+");
    }
    if key_mods.shift_key() {
      s.push_str("Shift+");
    }
    if key_mods.alt_key() {
      s.push_str("Alt+");
    }
    if key_mods.super_key() {
      s.push_str("Windows+");
    }
    match &self.key {
      KeyCode::KeyA => s.push('A'),
      KeyCode::KeyB => s.push('B'),
      KeyCode::KeyC => s.push('C'),
      KeyCode::KeyD => s.push('D'),
      KeyCode::KeyE => s.push('E'),
      KeyCode::KeyF => s.push('F'),
      KeyCode::KeyG => s.push('G'),
      KeyCode::KeyH => s.push('H'),
      KeyCode::KeyI => s.push('I'),
      KeyCode::KeyJ => s.push('J'),
      KeyCode::KeyK => s.push('K'),
      KeyCode::KeyL => s.push('L'),
      KeyCode::KeyM => s.push('M'),
      KeyCode::KeyN => s.push('N'),
      KeyCode::KeyO => s.push('O'),
      KeyCode::KeyP => s.push('P'),
      KeyCode::KeyQ => s.push('Q'),
      KeyCode::KeyR => s.push('R'),
      KeyCode::KeyS => s.push('S'),
      KeyCode::KeyT => s.push('T'),
      KeyCode::KeyU => s.push('U'),
      KeyCode::KeyV => s.push('V'),
      KeyCode::KeyW => s.push('W'),
      KeyCode::KeyX => s.push('X'),
      KeyCode::KeyY => s.push('Y'),
      KeyCode::KeyZ => s.push('Z'),
      KeyCode::Digit0 => s.push('0'),
      KeyCode::Digit1 => s.push('1'),
      KeyCode::Digit2 => s.push('2'),
      KeyCode::Digit3 => s.push('3'),
      KeyCode::Digit4 => s.push('4'),
      KeyCode::Digit5 => s.push('5'),
      KeyCode::Digit6 => s.push('6'),
      KeyCode::Digit7 => s.push('7'),
      KeyCode::Digit8 => s.push('8'),
      KeyCode::Digit9 => s.push('9'),
      KeyCode::Comma => s.push(','),
      KeyCode::Minus => s.push('-'),
      KeyCode::Period => s.push('.'),
      KeyCode::Space => s.push_str("Space"),
      KeyCode::Equal => s.push('='),
      KeyCode::Semicolon => s.push(';'),
      KeyCode::Slash => s.push('/'),
      KeyCode::Backslash => s.push('\\'),
      KeyCode::Quote => s.push('\''),
      KeyCode::Backquote => s.push('`'),
      KeyCode::BracketLeft => s.push('['),
      KeyCode::BracketRight => s.push(']'),
      KeyCode::Tab => s.push_str("Tab"),
      KeyCode::Escape => s.push_str("Esc"),
      KeyCode::Delete => s.push_str("Del"),
      KeyCode::Insert => s.push_str("Ins"),
      KeyCode::PageUp => s.push_str("PgUp"),
      KeyCode::PageDown => s.push_str("PgDn"),
      // These names match LibreOffice.
      KeyCode::ArrowLeft => s.push_str("Left"),
      KeyCode::ArrowRight => s.push_str("Right"),
      KeyCode::ArrowUp => s.push_str("Up"),
      KeyCode::ArrowDown => s.push_str("Down"),
      _ => s.push_str(&format!("{:?}", self.key)),
    };
    s
  }
}
