// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::{
  collections::HashMap,
  sync::{Mutex, MutexGuard},
};

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
  error::OsError as RootOsError,
  event::{Event, WindowEvent},
  keyboard::{KeyCode, ModifiersState},
  menu::Menu as RootMenu,
  menu::{MenuId, NativeMenuItem},
  window::WindowId as RootWindowId,
};

use super::{accelerator::register_accel, keyboard::key_to_vk, util, OsError, WindowId};

pub struct MenuEventHandler {
  window_id: Option<RootWindowId>,
  event_sender: Box<dyn Fn(Event<'static, ()>)>,
}

impl MenuEventHandler {
  pub fn new(
    event_sender: Box<dyn Fn(Event<'static, ()>)>,
    window_id: Option<RootWindowId>,
  ) -> MenuEventHandler {
    MenuEventHandler {
      window_id,
      event_sender,
    }
  }
  pub fn send_menu_event(&self, menu_id: u16) {
    (self.event_sender)(Event::MenuEvent {
      menu_id,
      window_id: self.window_id,
    });
  }

  pub fn send_event(&self, event: Event<'static, ()>) {
    (self.event_sender)(event);
  }
}

struct MenusData {
  menus: HashMap<MenuId, Menu>,
  custom_menu_items: HashMap<MenuId, CustomMenuItem>,
}

lazy_static! {
  static ref MENUS_DATA: Mutex<MenusData> = Mutex::new(MenusData {
    menus: HashMap::new(),
    custom_menu_items: HashMap::new()
  });
}

#[derive(Clone, Copy)]
enum MenuItem {
  Custom,
  Submenu,
  NativeItem,
}

pub struct Menu {
  id: MenuId,
  title: String,
  enabled: bool,
  items: Vec<(MenuItem, MenuId)>,
  hmenu: Option<HMENU>,
}

impl Menu {
  pub fn new(title: &str) -> Result<MenuId, RootOsError> {
    if let Ok(mut menus_data) = MENUS_DATA.lock() {
      let id = uuid::Uuid::new_v4().to_u128_le() as u16;
      menus_data.menus.insert(
        id,
        Menu {
          id,
          enabled: true,
          title: title.into(),
          items: Vec::new(),
          hmenu: None,
        },
      );
      Ok(id)
    } else {
      Err(os_error!(
        OsError::CreationError("Failed to register menu",)
      ))
    }
  }

  pub fn add_custom_item(pmenu_id: MenuId, item_id: MenuId) {
    if let Ok(mut menus_data) = MENUS_DATA.lock() {
      let mut pmenu_hmenu = None;
      {
        if let Some(menu) = menus_data.menus.get_mut(&pmenu_id) {
          menu.items.push((MenuItem::Custom, item_id));
          pmenu_hmenu = menu.hmenu;
        }
      }

      if let Some(item) = menus_data.custom_menu_items.get_mut(&item_id) {
        item.parent_menus.push(pmenu_id);
        if let Some(hmenu) = pmenu_hmenu {
          item.add_to_hmenu(hmenu);
        }
      }
    }
  }

  pub fn add_native_item(pmenu_id: MenuId, item: NativeMenuItem) {
    if let Ok(mut menus_data) = MENUS_DATA.lock() {
      let mut pmenu_hmenu = None;
      {
        if let Some(menu) = menus_data.menus.get_mut(&pmenu_id) {
          menu.items.push((MenuItem::NativeItem, item.id()));
          pmenu_hmenu = menu.hmenu;
        }
      }

      if let Some(hmenu) = pmenu_hmenu {
        item.add_to_hmenu(hmenu);
      }
    }
  }

  pub fn add_submenu(pmenu_id: MenuId, submenu_id: MenuId) {
    if let Ok(mut menus_data) = MENUS_DATA.lock() {
      if let Some(menu) = menus_data.menus.get_mut(&pmenu_id) {
        menu.items.push((MenuItem::Submenu, submenu_id));

        if let Some(hmenu) = menu.hmenu {
          if let Some(submenu) = menus_data.menus.get(&submenu_id) {
            let submenu_hmenu = Menu::make_submenu_hmenu(hmenu, submenu);
            Menu::add_to_hmenu(submenu_id, submenu_hmenu, &mut menus_data);
          }
        }
      }
    }
  }

  fn add_to_hmenu(menu_id: MenuId, hmenu: HMENU, menus_data: &mut MutexGuard<'_, MenusData>) {
    if let Some(menu) = menus_data.menus.get_mut(&menu_id) {
      for (mtype, id) in menu.items.clone() {
        match mtype {
          MenuItem::Custom => {
            if let Some(item) = menus_data.custom_menu_items.get_mut(&id) {
              item.add_to_hmenu(hmenu)
            }
          }
          MenuItem::Submenu => {
            if let Some(submenu) = menus_data.menus.get_mut(&id) {
              let submenu_hmenu = Menu::make_submenu_hmenu(hmenu, &submenu);
              submenu.hmenu = Some(submenu_hmenu);
              Menu::add_to_hmenu(id, submenu_hmenu, menus_data);
            }
          }
          MenuItem::NativeItem => {
            let item = NativeMenuItem::from_id(id);
            item.add_to_hmenu(hmenu);
          }
        }
      }
    }
  }

  fn make_submenu_hmenu(pmenu: HMENU, submenu: &Menu) -> HMENU {
    let hmenu = unsafe { CreateMenu() };
    // let child_accels = std::mem::take(&mut submenu.accels);
    // self.accels.extend(child_accels);

    let mut flags = MF_POPUP;
    if !submenu.enabled {
      flags |= MF_DISABLED;
    }
    unsafe {
      AppendMenuW(pmenu, flags, hmenu.0 as _, submenu.title.clone());
    }
    hmenu
  }
}

pub struct CustomMenuItem {
  id: u16,
  title: String,
  enabled: bool,
  selected: bool,
  accel: Option<Accelerator>,
  parent_menus: Vec<MenuId>,
}

impl CustomMenuItem {
  pub fn new(
    title: &str,
    enabled: bool,
    selected: bool,
    accel: Option<Accelerator>,
  ) -> Result<MenuId, RootOsError> {
    if let Ok(mut menus_data) = MENUS_DATA.lock() {
      let id = uuid::Uuid::new_v4().to_u128_le() as u16;
      menus_data.custom_menu_items.insert(
        id,
        Self {
          id,
          title: title.into(),
          enabled,
          selected,
          accel,
          parent_menus: Vec::new(),
        },
      );

      Ok(id)
    } else {
      Err(os_error!(OsError::CreationError(
        "Failed to register menu item",
      )))
    }
  }

  pub fn set_title(item_id: MenuId, title: &str) {
    if let Ok(mut menus_data) = MENUS_DATA.lock() {
      {
        if let Some(item) = menus_data.custom_menu_items.get_mut(&item_id) {
          item.title = title.into();
        }
      }

      if let Some(item) = menus_data.custom_menu_items.get(&item_id) {
        let info = MENUITEMINFOA {
          cbSize: std::mem::size_of::<MENUITEMINFOA>() as _,
          fMask: MIIM_STRING,
          // NOTE(amrbashir): The title must be a null-terminated string. Otherwise, it will display some gibberish characters at the end.
          dwTypeData: PSTR(format!("{}\0", title).as_ptr() as _),
          ..Default::default()
        };
        for menu_id in &item.parent_menus {
          if let Some(menu) = menus_data.menus.get(menu_id) {
            if let Some(hmenu) = menu.hmenu {
              unsafe {
                SetMenuItemInfoA(hmenu, item.id as _, false, &info);
              }
            }
          }
        }
      }
    }
  }

  pub fn set_enabled(item_id: MenuId, enabled: bool) {
    if let Ok(mut menus_data) = MENUS_DATA.lock() {
      {
        if let Some(item) = menus_data.custom_menu_items.get_mut(&item_id) {
          item.enabled = enabled;
        }
      }

      if let Some(item) = menus_data.custom_menu_items.get(&item_id) {
        for menu_id in &item.parent_menus {
          if let Some(menu) = menus_data.menus.get(menu_id) {
            if let Some(hmenu) = menu.hmenu {
              unsafe {
                EnableMenuItem(
                  hmenu,
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
    }
  }

  pub fn set_selected(item_id: MenuId, selected: bool) {
    if let Ok(mut menus_data) = MENUS_DATA.lock() {
      {
        if let Some(item) = menus_data.custom_menu_items.get_mut(&item_id) {
          item.selected = selected;
        }
      }

      if let Some(item) = menus_data.custom_menu_items.get(&item_id) {
        for menu_id in &item.parent_menus {
          if let Some(menu) = menus_data.menus.get(menu_id) {
            if let Some(hmenu) = menu.hmenu {
              unsafe {
                CheckMenuItem(
                  hmenu,
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
    // format title
    // if let Some(accelerators) = accelerators.clone() {
    //   anno_title.push('\t');
    //   format_hotkey(accelerators, &mut anno_title);
    // }
    unsafe {
      AppendMenuW(menu, flags, self.id as _, self.title.clone());
    }
    // add our accels
    // if let Some(accelerators) = accelerators {
    //   if let Some(accelerators) = convert_accelerator(menu_id.0, accelerators) {
    //     self.accels.insert(menu_id.0, AccelWrapper(accelerators));
    //   }
    // }
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
        AppendMenuW(hmenu, MF_STRING, self.id() as _, "&Hide\tCtrl+H");
      },
      NativeMenuItem::CloseWindow => unsafe {
        AppendMenuW(hmenu, MF_STRING, self.id() as _, "&Close\tAlt+F4");
      },
      NativeMenuItem::Quit => unsafe {
        AppendMenuW(hmenu, MF_STRING, self.id() as _, "&Quit");
      },
      _ => {}
    }
  }
}

const MENU_SUBCLASS_ID: usize = 4568;
pub fn set_for_window(menu: RootMenu, window: HWND, menu_handler: MenuEventHandler) -> HMENU {
  let sender = Box::into_raw(Box::new(menu_handler));
  let hmenubar = unsafe { CreateMenu() };

  if let Ok(mut menus_data) = MENUS_DATA.lock() {
    Menu::add_to_hmenu(menu.id(), hmenubar, &mut menus_data);
  }

  unsafe {
    SetWindowSubclass(window, Some(subclass_proc), MENU_SUBCLASS_ID, sender as _);
    SetMenu(window, hmenubar);
  }

  // if let Some(accels) = menu.accels() {
  //   register_accel(window, &accels);
  // }

  hmenubar
}

pub(crate) unsafe extern "system" fn subclass_proc(
  hwnd: HWND,
  msg: u32,
  wparam: WPARAM,
  lparam: LPARAM,
  _id: usize,
  subclass_input_ptr: usize,
) -> LRESULT {
  let subclass_input_ptr = subclass_input_ptr as *mut MenuEventHandler;
  let subclass_input = &*(subclass_input_ptr);

  if msg == WM_DESTROY {
    Box::from_raw(subclass_input_ptr);
  }

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
          subclass_input.send_event(Event::WindowEvent {
            window_id: RootWindowId(WindowId(hwnd.0)),
            event: WindowEvent::CloseRequested,
          });
        }
        _ if menu_id == NativeMenuItem::id(&NativeMenuItem::Quit) => {
          subclass_input.send_event(Event::LoopDestroyed);
        }

        _ => {
          let mut is_a_menu_event = false;
          {
            if let Ok(menus_data) = MENUS_DATA.lock() {
              if menus_data.custom_menu_items.get(&menu_id).is_some() {
                is_a_menu_event = true;
              }
            }
          }
          if is_a_menu_event {
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
