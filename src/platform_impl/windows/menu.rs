// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, fmt, sync::Mutex};

use windows::Win32::{
  Foundation::{HWND, LPARAM, LRESULT, PSTR, PWSTR, WPARAM},
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
  menu::{CustomMenuItem, MenuId, MenuItem, MenuType},
  window::WindowId as RootWindowId,
};

use super::{accelerator::register_accel, keyboard::key_to_vk, util, WindowId};

#[derive(Copy, Clone)]
struct AccelWrapper(ACCEL);
impl fmt::Debug for AccelWrapper {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
    f.pad(&format!(""))
  }
}

const CUT_ID: usize = 5001;
const COPY_ID: usize = 5002;
const PASTE_ID: usize = 5003;
const SELECT_ALL_ID: usize = 5004;
const HIDE_ID: usize = 5005;
const CLOSE_ID: usize = 5006;
const QUIT_ID: usize = 5007;
const MINIMIZE_ID: usize = 5008;

lazy_static! {
  static ref MENU_IDS: Mutex<Vec<u16>> = Mutex::new(vec![]);
}

pub struct MenuHandler {
  window_id: Option<RootWindowId>,
  menu_type: MenuType,
  event_sender: Box<dyn Fn(Event<'static, ()>)>,
}

impl MenuHandler {
  pub fn new(
    event_sender: Box<dyn Fn(Event<'static, ()>)>,
    menu_type: MenuType,
    window_id: Option<RootWindowId>,
  ) -> MenuHandler {
    MenuHandler {
      window_id,
      menu_type,
      event_sender,
    }
  }
  pub fn send_menu_event(&self, menu_id: u16) {
    (self.event_sender)(Event::MenuEvent {
      menu_id: MenuId(menu_id),
      origin: self.menu_type,
      window_id: self.window_id,
    });
  }

  pub fn send_event(&self, event: Event<'static, ()>) {
    (self.event_sender)(event);
  }
}

#[derive(Debug, Clone)]
pub struct MenuItemAttributes(pub(crate) u16, HMENU);

impl MenuItemAttributes {
  pub fn id(&self) -> MenuId {
    MenuId(self.0)
  }
  pub fn set_enabled(&mut self, enabled: bool) {
    unsafe {
      EnableMenuItem(
        self.1,
        self.0 as u32,
        match enabled {
          true => MF_ENABLED,
          false => MF_DISABLED,
        },
      );
    }
  }
  pub fn set_title(&mut self, title: &str) {
    unsafe {
      let info = MENUITEMINFOA {
        cbSize: std::mem::size_of::<MENUITEMINFOA>() as _,
        fMask: MIIM_STRING,
        dwTypeData: PSTR(String::from(title).as_mut_ptr()),
        ..Default::default()
      };

      SetMenuItemInfoA(self.1, self.0 as u32, false, &info);
    }
  }
  pub fn set_selected(&mut self, selected: bool) {
    unsafe {
      CheckMenuItem(
        self.1,
        self.0 as u32,
        match selected {
          true => MF_CHECKED,
          false => MF_UNCHECKED,
        },
      );
    }
  }

  // todo: set custom icon to the menu item
  pub fn set_icon(&mut self, _icon: Vec<u8>) {}
}

#[derive(Debug, Clone)]
pub struct Menu {
  hmenu: HMENU,
  accels: HashMap<u16, AccelWrapper>,
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
      let hmenu = CreateMenu();
      Menu {
        hmenu,
        accels: HashMap::default(),
      }
    }
  }

  pub fn new_popup_menu() -> Self {
    unsafe {
      let hmenu = CreatePopupMenu();
      Menu {
        hmenu,
        accels: HashMap::default(),
      }
    }
  }

  pub fn hmenu(&self) -> HMENU {
    self.hmenu
  }

  // Get the accels table
  pub(crate) fn accels(&self) -> Option<Vec<ACCEL>> {
    if self.accels.is_empty() {
      return None;
    }
    Some(self.accels.values().cloned().map(|d| d.0).collect())
  }

  pub fn add_item(
    &mut self,
    menu_id: MenuId,
    title: &str,
    accelerators: Option<Accelerator>,
    enabled: bool,
    selected: bool,
    _menu_type: MenuType,
  ) -> CustomMenuItem {
    unsafe {
      let mut flags = MF_STRING;
      if !enabled {
        flags |= MF_GRAYED;
      }
      if selected {
        flags |= MF_CHECKED;
      }

      let mut anno_title = title.to_string();
      // format title
      if let Some(accelerators) = accelerators.clone() {
        anno_title.push('\t');
        format_hotkey(accelerators, &mut anno_title);
      }

      AppendMenuW(self.hmenu, flags, menu_id.0 as _, anno_title);

      // add our accels
      if let Some(accelerators) = accelerators {
        if let Some(accelerators) = convert_accelerator(menu_id.0, accelerators) {
          self.accels.insert(menu_id.0, AccelWrapper(accelerators));
        }
      }
      MENU_IDS.lock().unwrap().push(menu_id.0 as _);
      CustomMenuItem(MenuItemAttributes(menu_id.0, self.hmenu))
    }
  }

  pub fn add_submenu(&mut self, title: &str, enabled: bool, mut submenu: Menu) {
    unsafe {
      let child_accels = std::mem::take(&mut submenu.accels);
      self.accels.extend(child_accels);

      let mut flags = MF_POPUP;
      if !enabled {
        flags |= MF_DISABLED;
      }

      AppendMenuW(self.hmenu, flags, submenu.hmenu().0 as usize, title);
    }
  }

  pub fn add_native_item(
    &mut self,
    item: MenuItem,
    _menu_type: MenuType,
  ) -> Option<CustomMenuItem> {
    match item {
      MenuItem::Separator => {
        unsafe {
          AppendMenuW(self.hmenu, MF_SEPARATOR, 0, PWSTR::default());
        };
      }
      MenuItem::Cut => unsafe {
        AppendMenuW(self.hmenu, MF_STRING, CUT_ID, "&Cut\tCtrl+X");
      },
      MenuItem::Copy => unsafe {
        AppendMenuW(self.hmenu, MF_STRING, COPY_ID, "&Copy\tCtrl+C");
      },
      MenuItem::Paste => unsafe {
        AppendMenuW(self.hmenu, MF_STRING, PASTE_ID, "&Paste\tCtrl+V");
      },
      MenuItem::SelectAll => unsafe {
        AppendMenuW(self.hmenu, MF_STRING, SELECT_ALL_ID, "&Select all\tCtrl+A");
      },
      MenuItem::Hide => unsafe {
        AppendMenuW(self.hmenu, MF_STRING, HIDE_ID, "&Hide\tCtrl+H");
      },
      MenuItem::CloseWindow => unsafe {
        AppendMenuW(self.hmenu, MF_STRING, CLOSE_ID, "&Close\tAlt+F4");
      },
      MenuItem::Quit => unsafe {
        AppendMenuW(self.hmenu, MF_STRING, QUIT_ID, "&Quit");
      },
      MenuItem::Minimize => unsafe {
        AppendMenuW(self.hmenu, MF_STRING, MINIMIZE_ID, "&Minimize");
      },
      // FIXME: create all shortcuts of MenuItem if possible...
      // like linux?
      _ => (),
    };

    None
  }
}

/*
  Disabled as menu's seems to be linked to the app
  so they are dropped when the app closes.
  see discussion here;

  https://github.com/tauri-apps/tao/pull/106#issuecomment-880034210

  impl Drop for Menu {
    fn drop(&mut self) {
      unsafe {
        DestroyMenu(self.hmenu);
      }
    }
  }
*/

const MENU_SUBCLASS_ID: usize = 4568;

pub fn initialize(menu_builder: Menu, window: HWND, menu_handler: MenuHandler) -> HMENU {
  let sender: *mut MenuHandler = Box::into_raw(Box::new(menu_handler));
  let menu = menu_builder.hmenu();

  unsafe {
    SetWindowSubclass(window, Some(subclass_proc), MENU_SUBCLASS_ID, sender as _);
    SetMenu(window, menu);
  }

  if let Some(accels) = menu_builder.accels() {
    register_accel(window, &accels);
  }

  menu
}

pub(crate) unsafe extern "system" fn subclass_proc(
  hwnd: HWND,
  msg: u32,
  wparam: WPARAM,
  lparam: LPARAM,
  _id: usize,
  subclass_input_ptr: usize,
) -> LRESULT {
  let subclass_input_ptr = subclass_input_ptr as *mut MenuHandler;
  let subclass_input = &*(subclass_input_ptr);

  if msg == WM_DESTROY {
    Box::from_raw(subclass_input_ptr);
  }

  match msg {
    win32wm::WM_COMMAND => {
      match wparam.0 {
        CUT_ID => {
          execute_edit_command(EditCommand::Cut);
        }
        COPY_ID => {
          execute_edit_command(EditCommand::Copy);
        }
        PASTE_ID => {
          execute_edit_command(EditCommand::Paste);
        }
        SELECT_ALL_ID => {
          execute_edit_command(EditCommand::SelectAll);
        }
        HIDE_ID => {
          ShowWindow(hwnd, SW_HIDE);
        }
        CLOSE_ID => {
          subclass_input.send_event(Event::WindowEvent {
            window_id: RootWindowId(WindowId(hwnd.0)),
            event: WindowEvent::CloseRequested,
          });
        }
        QUIT_ID => {
          subclass_input.send_event(Event::LoopDestroyed);
          PostQuitMessage(0);
        }
        MINIMIZE_ID => {
          ShowWindow(hwnd, SW_MINIMIZE);
        }
        _ => {
          let menu_id = util::LOWORD(wparam.0 as u32);
          if MENU_IDS.lock().unwrap().contains(&menu_id) {
            subclass_input.send_menu_event(menu_id);
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

// Convert a hotkey to an accelerator.
fn convert_accelerator(id: u16, key: Accelerator) -> Option<ACCEL> {
  let mut virt_key = FVIRTKEY;
  let key_mods: ModifiersState = key.mods;
  if key_mods.control_key() {
    virt_key |= FCONTROL;
  }
  if key_mods.alt_key() {
    virt_key |= FALT;
  }
  if key_mods.shift_key() {
    virt_key |= FSHIFT;
  }

  let raw_key = if let Some(vk_code) = key_to_vk(&key.key) {
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
    dbg!("Failed to convert key {:?} into virtual key code", key.key);
    return None;
  };

  Some(ACCEL {
    fVirt: virt_key as u8,
    key: raw_key as u16,
    cmd: id,
  })
}

// Format the hotkey in a Windows-native way.
fn format_hotkey(key: Accelerator, s: &mut String) {
  let key_mods: ModifiersState = key.mods;
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
  match &key.key {
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
    _ => s.push_str(&format!("{:?}", key.key)),
  }
}
