// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, fmt, sync::Mutex};

use windows::{
  core::{PCWSTR, PWSTR},
  Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
      Input::KeyboardAndMouse::*,
      Shell::*,
      WindowsAndMessaging::{self as win32wm, *},
    },
  },
};

use crate::{
  accelerator::Accelerator,
  event::{Event, WindowEvent},
  icon::Icon,
  keyboard::{KeyCode, ModifiersState},
  menu::{CustomMenuItem, MenuId, MenuItem, MenuType},
  window::WindowId as RootWindowId,
};

use super::{accelerator::register_accel, keyboard::key_to_vk, util, WindowId};

#[derive(Copy, Clone)]
struct AccelWrapper(ACCEL);
impl fmt::Debug for AccelWrapper {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
    f.pad("")
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
pub struct MenuItemAttributes(pub(crate) u16, HMENU, Option<Accelerator>);

impl MenuItemAttributes {
  pub fn id(&self) -> MenuId {
    MenuId(self.0)
  }
  pub fn title(&self) -> String {
    unsafe {
      let mut mif = MENUITEMINFOW {
        cbSize: std::mem::size_of::<MENUITEMINFOW>() as _,
        fMask: MIIM_STRING,
        dwTypeData: PWSTR::null(),
        ..Default::default()
      };
      GetMenuItemInfoW(self.1, self.0 as u32, false, &mut mif);
      mif.cch += 1;
      mif.dwTypeData = PWSTR::from_raw(Vec::with_capacity(mif.cch as usize).as_mut_ptr());
      GetMenuItemInfoW(self.1, self.0 as u32, false, &mut mif);
      util::wchar_ptr_to_string(PCWSTR::from_raw(mif.dwTypeData.0))
        .split('\t')
        .next()
        .unwrap_or_default()
        .to_string()
    }
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
    let mut title = title.to_string();
    if let Some(accelerator) = &self.2 {
      title.push('\t');
      title.push_str(accelerator.to_string().as_str());
    }
    unsafe {
      let info = MENUITEMINFOW {
        cbSize: std::mem::size_of::<MENUITEMINFOW>() as _,
        fMask: MIIM_STRING,
        dwTypeData: PWSTR::from_raw(util::encode_wide(title).as_mut_ptr()),
        ..Default::default()
      };

      SetMenuItemInfoW(self.1, self.0 as u32, false, &info);
    }
  }
  pub fn set_selected(&mut self, selected: bool) {
    unsafe {
      CheckMenuItem(
        self.1,
        self.0 as u32,
        match selected {
          true => MF_CHECKED.0,
          false => MF_UNCHECKED.0,
        },
      );
    }
  }

  // todo: set custom icon to the menu item
  pub fn set_icon(&mut self, _icon: Icon) {}
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
      let hmenu = CreateMenu().unwrap_or_default();
      Menu {
        hmenu,
        accels: HashMap::default(),
      }
    }
  }

  pub fn new_popup_menu() -> Self {
    unsafe {
      let hmenu = CreatePopupMenu().unwrap_or_default();
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
    accelerator: Option<Accelerator>,
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

      let mut title = title.to_string();
      if let Some(accelerator) = &accelerator {
        title.push('\t');
        title.push_str(accelerator.to_string().as_str());
      }

      AppendMenuW(
        self.hmenu,
        flags,
        menu_id.0 as _,
        PCWSTR::from_raw(util::encode_wide(title).as_ptr()),
      );

      // add our accels
      if let Some(accelerators) = &accelerator {
        if let Some(accelerators) = accelerators.to_accel(menu_id.0) {
          self.accels.insert(menu_id.0, AccelWrapper(accelerators));
        }
      }
      MENU_IDS.lock().unwrap().push(menu_id.0 as _);
      CustomMenuItem(MenuItemAttributes(menu_id.0, self.hmenu, accelerator))
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

      let title = util::encode_wide(title);
      AppendMenuW(
        self.hmenu,
        flags,
        submenu.hmenu().0 as usize,
        PCWSTR::from_raw(title.as_ptr()),
      );
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
          AppendMenuW(self.hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        };
      }
      MenuItem::Cut => unsafe {
        AppendMenuW(
          self.hmenu,
          MF_STRING,
          CUT_ID,
          PCWSTR::from_raw(util::encode_wide("&Cut\tCtrl+X").as_ptr()),
        );
      },
      MenuItem::Copy => unsafe {
        AppendMenuW(
          self.hmenu,
          MF_STRING,
          COPY_ID,
          PCWSTR::from_raw(util::encode_wide("&Copy\tCtrl+C").as_ptr()),
        );
      },
      MenuItem::Paste => unsafe {
        AppendMenuW(
          self.hmenu,
          MF_STRING,
          PASTE_ID,
          PCWSTR::from_raw(util::encode_wide("&Paste\tCtrl+V").as_ptr()),
        );
      },
      MenuItem::SelectAll => unsafe {
        AppendMenuW(
          self.hmenu,
          MF_STRING,
          SELECT_ALL_ID,
          PCWSTR::from_raw(util::encode_wide("&Select all\tCtrl+A").as_ptr()),
        );
      },
      MenuItem::Hide => unsafe {
        AppendMenuW(
          self.hmenu,
          MF_STRING,
          HIDE_ID,
          PCWSTR::from_raw(util::encode_wide("&Hide\tCtrl+H").as_ptr()),
        );
      },
      MenuItem::CloseWindow => unsafe {
        AppendMenuW(
          self.hmenu,
          MF_STRING,
          CLOSE_ID,
          PCWSTR::from_raw(util::encode_wide("&Close\tAlt+F4").as_ptr()),
        );
      },
      MenuItem::Quit => unsafe {
        AppendMenuW(
          self.hmenu,
          MF_STRING,
          QUIT_ID,
          PCWSTR::from_raw(util::encode_wide("&Quit").as_ptr()),
        );
      },
      MenuItem::Minimize => unsafe {
        AppendMenuW(
          self.hmenu,
          MF_STRING,
          MINIMIZE_ID,
          PCWSTR::from_raw(util::encode_wide("&Minimize").as_ptr()),
        );
      },
      // FIXME: create all shortcuts of MenuItem if possible...
      // like linux?
      _ => (),
    };

    None
  }
}

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
    drop(Box::from_raw(subclass_input_ptr));
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
  let key = VIRTUAL_KEY(match command {
    EditCommand::Copy => 0x43,      // c
    EditCommand::Cut => 0x58,       // x
    EditCommand::Paste => 0x56,     // v
    EditCommand::SelectAll => 0x41, // a
  });

  unsafe {
    let mut inputs: [INPUT; 4] = std::mem::zeroed();
    inputs[0].r#type = INPUT_KEYBOARD;
    inputs[0].Anonymous.ki.wVk = VK_CONTROL;
    inputs[2].Anonymous.ki.dwFlags = Default::default();

    inputs[1].r#type = INPUT_KEYBOARD;
    inputs[1].Anonymous.ki.wVk = key;
    inputs[2].Anonymous.ki.dwFlags = Default::default();

    inputs[2].r#type = INPUT_KEYBOARD;
    inputs[2].Anonymous.ki.wVk = key;
    inputs[2].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

    inputs[3].r#type = INPUT_KEYBOARD;
    inputs[3].Anonymous.ki.wVk = VK_CONTROL;
    inputs[3].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

    SendInput(&inputs, std::mem::size_of::<INPUT>() as _);
  }
}

impl Accelerator {
  // Convert a hotkey to an accelerator.
  fn to_accel(&self, menu_id: u16) -> Option<ACCEL> {
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
      let mod_code = vk_code.0 >> 8;
      if mod_code & 0x1 != 0 {
        virt_key |= FSHIFT;
      }
      if mod_code & 0x02 != 0 {
        virt_key |= FCONTROL;
      }
      if mod_code & 0x04 != 0 {
        virt_key |= FALT;
      }
      vk_code.0 & 0x00ff
    } else {
      dbg!("Failed to convert key {:?} into virtual key code", self.key);
      return None;
    };

    Some(ACCEL {
      fVirt: virt_key,
      key: raw_key as u16,
      cmd: menu_id,
    })
  }
}

impl fmt::Display for Accelerator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let key_mods: ModifiersState = self.mods;
    if key_mods.control_key() {
      write!(f, "Ctrl+")?;
    }
    if key_mods.shift_key() {
      write!(f, "Shift+")?;
    }
    if key_mods.alt_key() {
      write!(f, "Alt+")?;
    }
    if key_mods.super_key() {
      write!(f, "Windows+")?;
    }
    match &self.key {
      KeyCode::KeyA => write!(f, "A"),
      KeyCode::KeyB => write!(f, "B"),
      KeyCode::KeyC => write!(f, "C"),
      KeyCode::KeyD => write!(f, "D"),
      KeyCode::KeyE => write!(f, "E"),
      KeyCode::KeyF => write!(f, "F"),
      KeyCode::KeyG => write!(f, "G"),
      KeyCode::KeyH => write!(f, "H"),
      KeyCode::KeyI => write!(f, "I"),
      KeyCode::KeyJ => write!(f, "J"),
      KeyCode::KeyK => write!(f, "K"),
      KeyCode::KeyL => write!(f, "L"),
      KeyCode::KeyM => write!(f, "M"),
      KeyCode::KeyN => write!(f, "N"),
      KeyCode::KeyO => write!(f, "O"),
      KeyCode::KeyP => write!(f, "P"),
      KeyCode::KeyQ => write!(f, "Q"),
      KeyCode::KeyR => write!(f, "R"),
      KeyCode::KeyS => write!(f, "S"),
      KeyCode::KeyT => write!(f, "T"),
      KeyCode::KeyU => write!(f, "U"),
      KeyCode::KeyV => write!(f, "V"),
      KeyCode::KeyW => write!(f, "W"),
      KeyCode::KeyX => write!(f, "X"),
      KeyCode::KeyY => write!(f, "Y"),
      KeyCode::KeyZ => write!(f, "Z"),
      KeyCode::Digit0 => write!(f, "0"),
      KeyCode::Digit1 => write!(f, "1"),
      KeyCode::Digit2 => write!(f, "2"),
      KeyCode::Digit3 => write!(f, "3"),
      KeyCode::Digit4 => write!(f, "4"),
      KeyCode::Digit5 => write!(f, "5"),
      KeyCode::Digit6 => write!(f, "6"),
      KeyCode::Digit7 => write!(f, "7"),
      KeyCode::Digit8 => write!(f, "8"),
      KeyCode::Digit9 => write!(f, "9"),
      KeyCode::Comma => write!(f, ","),
      KeyCode::Minus => write!(f, "-"),
      KeyCode::Plus => write!(f, "+"),
      KeyCode::Period => write!(f, "."),
      KeyCode::Space => write!(f, "Space"),
      KeyCode::Equal => write!(f, "="),
      KeyCode::Semicolon => write!(f, ";"),
      KeyCode::Slash => write!(f, "/"),
      KeyCode::Backslash => write!(f, "\\"),
      KeyCode::Quote => write!(f, "\'"),
      KeyCode::Backquote => write!(f, "`"),
      KeyCode::BracketLeft => write!(f, "["),
      KeyCode::BracketRight => write!(f, "]"),
      KeyCode::Tab => write!(f, "Tab"),
      KeyCode::Escape => write!(f, "Esc"),
      KeyCode::Delete => write!(f, "Del"),
      KeyCode::Insert => write!(f, "Ins"),
      KeyCode::PageUp => write!(f, "PgUp"),
      KeyCode::PageDown => write!(f, "PgDn"),
      // These names match LibreOffice.
      KeyCode::ArrowLeft => write!(f, "Left"),
      KeyCode::ArrowRight => write!(f, "Right"),
      KeyCode::ArrowUp => write!(f, "Up"),
      KeyCode::ArrowDown => write!(f, "Down"),
      _ => write!(f, "{:?}", self.key),
    }
  }
}
