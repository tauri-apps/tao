use std::{
  char, collections::HashSet, ffi::OsString, mem::MaybeUninit, os::windows::ffi::OsStringExt,
  sync::MutexGuard,
};

use windows::Win32::{
  Foundation::{HWND, LPARAM, LRESULT, WPARAM},
  UI::{
    Input::KeyboardAndMouse::{self as win32km, *},
    TextServices::HKL,
    WindowsAndMessaging::{self as win32wm, *},
  },
};

use unicode_segmentation::UnicodeSegmentation;

use crate::{
  event::{ElementState, KeyEvent},
  keyboard::{Key, KeyCode, KeyLocation, NativeKeyCode},
  platform_impl::platform::{
    event_loop::ProcResult,
    keyboard_layout::{get_or_insert_str, Layout, LayoutCache, WindowsModifiers, LAYOUT_CACHE},
    KeyEventExtra,
  },
};

pub fn is_msg_keyboard_related(msg: u32) -> bool {
  let is_keyboard_msg = WM_KEYFIRST <= msg && msg <= WM_KEYLAST;

  is_keyboard_msg || msg == WM_SETFOCUS || msg == WM_KILLFOCUS
}

pub type ExScancode = u16;

pub struct MessageAsKeyEvent {
  pub event: KeyEvent,
  pub is_synthetic: bool,
}

/// Stores information required to make `KeyEvent`s.
///
/// A single Tao `KeyEvent` contains information which the Windows API passes to the application
/// in multiple window messages. In other words: a Tao `KeyEvent` cannot be built from a single
/// window message. Therefore, this type keeps track of certain information from previous events so
/// that a `KeyEvent` can be constructed when the last event related to a keypress is received.
///
/// `PeekMessage` is sometimes used to determine whether the next window message still belongs to the
/// current keypress. If it doesn't and the current state represents a key event waiting to be
/// dispatched, then said event is considered complete and is dispatched.
///
/// The sequence of window messages for a key press event is the following:
/// - Exactly one WM_KEYDOWN / WM_SYSKEYDOWN
/// - Zero or one WM_DEADCHAR / WM_SYSDEADCHAR
/// - Zero or more WM_CHAR / WM_SYSCHAR. These messages each come with a UTF-16 code unit which when
///   put together in the sequence they arrived in, forms the text which is the result of pressing the
///   key.
///
/// Key release messages are a bit different due to the fact that they don't contribute to
/// text input. The "sequence" only consists of one WM_KEYUP / WM_SYSKEYUP event.
#[derive(Default)]
pub struct KeyEventBuilder {
  event_info: Option<PartialKeyEventInfo>,
}
impl KeyEventBuilder {
  /// Call this function for every window message.
  /// Returns Some() if this window message completes a KeyEvent.
  /// Returns None otherwise.
  pub(crate) fn process_message(
    &mut self,
    hwnd: HWND,
    msg_kind: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    result: &mut ProcResult,
  ) -> Vec<MessageAsKeyEvent> {
    match msg_kind {
      win32wm::WM_SETFOCUS => {
        // synthesize keydown events
        let kbd_state = get_async_kbd_state();
        let key_events = self.synthesize_kbd_state(ElementState::Pressed, &kbd_state);
        if !key_events.is_empty() {
          return key_events;
        }
      }
      win32wm::WM_KILLFOCUS => {
        // sythesize keyup events
        let kbd_state = get_kbd_state();
        let key_events = self.synthesize_kbd_state(ElementState::Released, &kbd_state);
        if !key_events.is_empty() {
          return key_events;
        }
      }
      win32wm::WM_KEYDOWN | win32wm::WM_SYSKEYDOWN => {
        if msg_kind == WM_SYSKEYDOWN && wparam.0 == usize::from(VK_F4) {
          // Don't dispatch Alt+F4 to the application.
          // This is handled in `event_loop.rs`
          return vec![];
        }
        *result = ProcResult::Value(LRESULT(0));

        let mut layouts = LAYOUT_CACHE.lock().unwrap();
        let event_info =
          PartialKeyEventInfo::from_message(wparam, lparam, ElementState::Pressed, &mut layouts);

        let mut next_msg = MaybeUninit::uninit();
        let peek_retval = unsafe {
          PeekMessageW(
            next_msg.as_mut_ptr(),
            hwnd,
            WM_KEYFIRST,
            WM_KEYLAST,
            PM_NOREMOVE,
          )
        };
        let has_next_key_message = peek_retval.as_bool();
        self.event_info = None;
        let mut finished_event_info = Some(event_info);
        if has_next_key_message {
          let next_msg = unsafe { next_msg.assume_init() };
          let next_msg_kind = next_msg.message;
          let next_belongs_to_this = !matches!(
            next_msg_kind,
            win32wm::WM_KEYDOWN | win32wm::WM_SYSKEYDOWN | win32wm::WM_KEYUP | win32wm::WM_SYSKEYUP
          );
          if next_belongs_to_this {
            self.event_info = finished_event_info.take();
          } else {
            let (_, layout) = layouts.get_current_layout();
            let is_fake = {
              let curr_event = finished_event_info.as_ref().unwrap();
              is_current_fake(curr_event, next_msg, layout)
            };
            if is_fake {
              finished_event_info = None;
            }
          }
        }
        if let Some(event_info) = finished_event_info {
          let ev = event_info.finalize(&mut layouts.strings);
          return vec![MessageAsKeyEvent {
            event: ev,
            is_synthetic: false,
          }];
        }
      }
      win32wm::WM_DEADCHAR | win32wm::WM_SYSDEADCHAR => {
        *result = ProcResult::Value(LRESULT(0));
        // At this point, we know that there isn't going to be any more events related to
        // this key press
        let event_info = self.event_info.take().unwrap();
        let mut layouts = LAYOUT_CACHE.lock().unwrap();
        let ev = event_info.finalize(&mut layouts.strings);
        return vec![MessageAsKeyEvent {
          event: ev,
          is_synthetic: false,
        }];
      }
      win32wm::WM_CHAR | win32wm::WM_SYSCHAR => {
        if self.event_info.is_none() {
          trace!("Received a CHAR message but no `event_info` was available. The message is probably IME, returning.");
          return vec![];
        }
        *result = ProcResult::Value(LRESULT(0));
        let is_high_surrogate = (0xD800..=0xDBFF).contains(&wparam.0);
        let is_low_surrogate = (0xDC00..=0xDFFF).contains(&wparam.0);

        let is_utf16 = is_high_surrogate || is_low_surrogate;

        let more_char_coming;
        unsafe {
          let mut next_msg = MaybeUninit::uninit();
          let has_message = PeekMessageW(
            next_msg.as_mut_ptr(),
            hwnd,
            WM_KEYFIRST,
            WM_KEYLAST,
            PM_NOREMOVE,
          );
          let has_message = has_message.as_bool();
          if !has_message {
            more_char_coming = false;
          } else {
            let next_msg = next_msg.assume_init().message;
            if next_msg == WM_CHAR || next_msg == WM_SYSCHAR {
              more_char_coming = true;
            } else {
              more_char_coming = false;
            }
          }
        }

        if is_utf16 {
          if let Some(ev_info) = self.event_info.as_mut() {
            ev_info.utf16parts.push(wparam.0 as u16);
          }
        } else {
          // In this case, wparam holds a UTF-32 character.
          // Let's encode it as UTF-16 and append it to the end of `utf16parts`
          let utf16parts = match self.event_info.as_mut() {
            Some(ev_info) => &mut ev_info.utf16parts,
            None => {
              warn!("The event_info was None when it was expected to be some");
              return vec![];
            }
          };
          let start_offset = utf16parts.len();
          let new_size = utf16parts.len() + 2;
          utf16parts.resize(new_size, 0);
          if let Some(ch) = char::from_u32(wparam.0 as u32) {
            let encode_len = ch.encode_utf16(&mut utf16parts[start_offset..]).len();
            let new_size = start_offset + encode_len;
            utf16parts.resize(new_size, 0);
          }
        }
        if !more_char_coming {
          let mut event_info = match self.event_info.take() {
            Some(ev_info) => ev_info,
            None => {
              warn!("The event_info was None when it was expected to be some");
              return vec![];
            }
          };
          let mut layouts = LAYOUT_CACHE.lock().unwrap();
          // It's okay to call `ToUnicode` here, because at this point the dead key
          // is already consumed by the character.
          let kbd_state = get_kbd_state();
          let mod_state = WindowsModifiers::active_modifiers(&kbd_state);

          let (_, layout) = layouts.get_current_layout();
          let ctrl_on;
          if layout.has_alt_graph {
            let alt_on = mod_state.contains(WindowsModifiers::ALT);
            ctrl_on = !alt_on && mod_state.contains(WindowsModifiers::CONTROL)
          } else {
            ctrl_on = mod_state.contains(WindowsModifiers::CONTROL)
          }

          // If Ctrl is not pressed, just use the text with all
          // modifiers because that already consumed the dead key. Otherwise,
          // we would interpret the character incorrectly, missing the dead key.
          if !ctrl_on {
            event_info.text = PartialText::System(event_info.utf16parts.clone());
          } else {
            let mod_no_ctrl = mod_state.remove_only_ctrl();
            let num_lock_on = kbd_state[usize::from(VK_NUMLOCK)] & 1 != 0;
            let vkey = event_info.vkey;
            let scancode = event_info.scancode;
            let keycode = event_info.code;
            let key = layout.get_key(mod_no_ctrl, num_lock_on, vkey, scancode, keycode);
            event_info.text = PartialText::Text(key.to_text());
          }
          let ev = event_info.finalize(&mut layouts.strings);
          return vec![MessageAsKeyEvent {
            event: ev,
            is_synthetic: false,
          }];
        }
      }
      win32wm::WM_KEYUP | win32wm::WM_SYSKEYUP => {
        *result = ProcResult::Value(LRESULT(0));

        let mut layouts = LAYOUT_CACHE.lock().unwrap();
        let event_info =
          PartialKeyEventInfo::from_message(wparam, lparam, ElementState::Released, &mut layouts);
        let mut next_msg = MaybeUninit::uninit();
        let peek_retval = unsafe {
          PeekMessageW(
            next_msg.as_mut_ptr(),
            hwnd,
            WM_KEYFIRST,
            WM_KEYLAST,
            PM_NOREMOVE,
          )
        };
        let has_next_key_message = peek_retval.as_bool();
        let mut valid_event_info = Some(event_info);
        if has_next_key_message {
          let next_msg = unsafe { next_msg.assume_init() };
          let (_, layout) = layouts.get_current_layout();
          let is_fake = {
            let event_info = valid_event_info.as_ref().unwrap();
            is_current_fake(event_info, next_msg, layout)
          };
          if is_fake {
            valid_event_info = None;
          }
        }
        if let Some(event_info) = valid_event_info {
          let event = event_info.finalize(&mut layouts.strings);
          return vec![MessageAsKeyEvent {
            event,
            is_synthetic: false,
          }];
        }
      }
      _ => (),
    }

    Vec::new()
  }

  fn synthesize_kbd_state(
    &mut self,
    key_state: ElementState,
    kbd_state: &[u8; 256],
  ) -> Vec<MessageAsKeyEvent> {
    let mut key_events = Vec::new();

    let mut layouts = LAYOUT_CACHE.lock().unwrap();
    let (locale_id, _) = layouts.get_current_layout();

    let is_key_pressed = |vk: VIRTUAL_KEY| &kbd_state[usize::from(vk)] & 0x80 != 0;

    // Is caps-lock active? Note that this is different from caps-lock
    // being held down.
    let caps_lock_on = kbd_state[usize::from(VK_CAPITAL)] & 1 != 0;
    let num_lock_on = kbd_state[usize::from(VK_NUMLOCK)] & 1 != 0;

    // We are synthesizing the press event for caps-lock first for the following reasons:
    // 1. If caps-lock is *not* held down but *is* active, then we have to
    //    synthesize all printable keys, respecting the caps-lock state.
    // 2. If caps-lock is held down, we could choose to sythesize its
    //    keypress after every other key, in which case all other keys *must*
    //    be sythesized as if the caps-lock state was be the opposite
    //    of what it currently is.
    // --
    // For the sake of simplicity we are choosing to always sythesize
    // caps-lock first, and always use the current caps-lock state
    // to determine the produced text
    if is_key_pressed(VK_CAPITAL) {
      let event = self.create_synthetic(
        VK_CAPITAL,
        key_state,
        caps_lock_on,
        num_lock_on,
        locale_id,
        &mut layouts,
      );
      if let Some(event) = event {
        key_events.push(event);
      }
    }
    let do_non_modifier = |key_events: &mut Vec<_>, layouts: &mut _| {
      for vk in 0..256 {
        let vk = vk as VIRTUAL_KEY;
        match vk {
          _ if vk == VK_CONTROL
            || vk == VK_LCONTROL
            || vk == VK_RCONTROL
            || vk == VK_SHIFT
            || vk == VK_LSHIFT
            || vk == VK_RSHIFT
            || vk == VK_MENU
            || vk == VK_LMENU
            || vk == VK_RMENU
            || vk == VK_CAPITAL =>
          {
            continue
          }
          _ => (),
        }
        if !is_key_pressed(vk) {
          continue;
        }
        let event = self.create_synthetic(
          vk,
          key_state,
          caps_lock_on,
          num_lock_on,
          locale_id as HKL,
          layouts,
        );
        if let Some(event) = event {
          key_events.push(event);
        }
      }
    };
    let do_modifier = |key_events: &mut Vec<_>, layouts: &mut _| {
      const CLEAR_MODIFIER_VKS: [VIRTUAL_KEY; 6] = [
        VK_LCONTROL,
        VK_LSHIFT,
        VK_LMENU,
        VK_RCONTROL,
        VK_RSHIFT,
        VK_RMENU,
      ];
      for vk in CLEAR_MODIFIER_VKS.iter() {
        if is_key_pressed(*vk) {
          let event = self.create_synthetic(
            *vk,
            key_state,
            caps_lock_on,
            num_lock_on,
            locale_id as HKL,
            layouts,
          );
          if let Some(event) = event {
            key_events.push(event);
          }
        }
      }
    };

    // Be cheeky and sequence modifier and non-modifier
    // key events such that non-modifier keys are not affected
    // by modifiers (except for caps-lock)
    match key_state {
      ElementState::Pressed => {
        do_non_modifier(&mut key_events, &mut layouts);
        do_modifier(&mut key_events, &mut layouts);
      }
      ElementState::Released => {
        do_modifier(&mut key_events, &mut layouts);
        do_non_modifier(&mut key_events, &mut layouts);
      }
    }

    key_events
  }

  fn create_synthetic(
    &self,
    vk: VIRTUAL_KEY,
    key_state: ElementState,
    caps_lock_on: bool,
    num_lock_on: bool,
    locale_id: HKL,
    layouts: &mut MutexGuard<'_, LayoutCache>,
  ) -> Option<MessageAsKeyEvent> {
    let scancode = unsafe { MapVirtualKeyExW(u32::from(vk), MAPVK_VK_TO_VSC_EX, locale_id) };
    if scancode == 0 {
      return None;
    }
    let scancode = scancode as ExScancode;
    let code = KeyCode::from_scancode(scancode as u32);
    let mods = if caps_lock_on {
      WindowsModifiers::CAPS_LOCK
    } else {
      WindowsModifiers::empty()
    };
    let layout = layouts.layouts.get(&locale_id.0).unwrap();
    let logical_key = layout.get_key(mods, num_lock_on, vk, scancode, code);
    let key_without_modifiers =
      layout.get_key(WindowsModifiers::empty(), false, vk, scancode, code);
    let text;
    if key_state == ElementState::Pressed {
      text = logical_key.to_text();
    } else {
      text = None;
    }
    let event_info = PartialKeyEventInfo {
      vkey: vk,
      logical_key: PartialLogicalKey::This(logical_key.clone()),
      key_without_modifiers,
      key_state,
      scancode,
      is_repeat: false,
      code,
      location: get_location(scancode, locale_id),
      utf16parts: Vec::with_capacity(8),
      text: PartialText::Text(text),
    };

    let mut event = event_info.finalize(&mut layouts.strings);
    event.logical_key = logical_key;
    event.platform_specific.text_with_all_modifiers = text;
    Some(MessageAsKeyEvent {
      event,
      is_synthetic: true,
    })
  }
}

enum PartialText {
  // Unicode
  System(Vec<u16>),
  Text(Option<&'static str>),
}

enum PartialLogicalKey {
  /// Use the text provided by the WM_CHAR messages and report that as a `Character` variant. If
  /// the text consists of multiple grapheme clusters (user-precieved characters) that means that
  /// dead key could not be combined with the second input, and in that case we should fall back
  /// to using what would have without a dead-key input.
  TextOr(Key<'static>),

  /// Use the value directly provided by this variant
  This(Key<'static>),
}

struct PartialKeyEventInfo {
  vkey: VIRTUAL_KEY,
  scancode: ExScancode,
  key_state: ElementState,
  is_repeat: bool,
  code: KeyCode,
  location: KeyLocation,
  logical_key: PartialLogicalKey,

  key_without_modifiers: Key<'static>,

  /// The UTF-16 code units of the text that was produced by the keypress event.
  /// This take all modifiers into account. Including CTRL
  utf16parts: Vec<u16>,

  text: PartialText,
}

impl PartialKeyEventInfo {
  fn from_message(
    wparam: WPARAM,
    lparam: LPARAM,
    state: ElementState,
    layouts: &mut MutexGuard<'_, LayoutCache>,
  ) -> Self {
    const NO_MODS: WindowsModifiers = WindowsModifiers::empty();

    let (_, layout) = layouts.get_current_layout();
    let lparam_struct = destructure_key_lparam(lparam);
    let scancode;
    let vkey = wparam.0 as VIRTUAL_KEY;
    if lparam_struct.scancode == 0 {
      // In some cases (often with media keys) the device reports a scancode of 0 but a
      // valid virtual key. In these cases we obtain the scancode from the virtual key.
      scancode =
        unsafe { MapVirtualKeyExW(u32::from(vkey), MAPVK_VK_TO_VSC_EX, layout.hkl) as u16 };
    } else {
      scancode = new_ex_scancode(lparam_struct.scancode, lparam_struct.extended);
    }
    let code = KeyCode::from_scancode(scancode as u32);
    let location = get_location(scancode, layout.hkl);

    let kbd_state = get_kbd_state();
    let mods = WindowsModifiers::active_modifiers(&kbd_state);
    let mods_without_ctrl = mods.remove_only_ctrl();
    let num_lock_on = kbd_state[VK_NUMLOCK as usize] & 1 != 0;

    // On Windows Ctrl+NumLock = Pause (and apparently Ctrl+Pause -> NumLock). In these cases
    // the KeyCode still stores the real key, so in the name of consistency across platforms, we
    // circumvent this mapping and force the key values to match the keycode.
    // For more on this, read the article by Raymond Chen, titled:
    // "Why does Ctrl+ScrollLock cancel dialogs?"
    // https://devblogs.microsoft.com/oldnewthing/20080211-00/?p=23503
    let code_as_key = if mods.contains(WindowsModifiers::CONTROL) {
      match code {
        KeyCode::NumLock => Some(Key::NumLock),
        KeyCode::Pause => Some(Key::Pause),
        _ => None,
      }
    } else {
      None
    };

    let preliminary_logical_key =
      layout.get_key(mods_without_ctrl, num_lock_on, vkey, scancode, code);
    let key_is_char = matches!(preliminary_logical_key, Key::Character(_));
    let is_pressed = state == ElementState::Pressed;

    let logical_key = if let Some(key) = code_as_key.clone() {
      PartialLogicalKey::This(key)
    } else if is_pressed && key_is_char && !mods.contains(WindowsModifiers::CONTROL) {
      // In some cases we want to use the UNICHAR text for logical_key in order to allow
      // dead keys to have an effect on the character reported by `logical_key`.
      PartialLogicalKey::TextOr(preliminary_logical_key)
    } else {
      PartialLogicalKey::This(preliminary_logical_key)
    };
    let key_without_modifiers = if let Some(key) = code_as_key {
      key
    } else {
      match layout.get_key(NO_MODS, false, vkey, scancode, code) {
        // We convert dead keys into their character.
        // The reason for this is that `key_without_modifiers` is designed for key-bindings,
        // but the US International layout treats `'` (apostrophe) as a dead key and the
        // reguar US layout treats it a character. In order for a single binding
        // configuration to work with both layouts, we forward each dead key as a character.
        Key::Dead(k) => {
          if let Some(ch) = k {
            // I'm avoiding the heap allocation. I don't want to talk about it :(
            let mut utf8 = [0; 4];
            let s = ch.encode_utf8(&mut utf8);
            let static_str = get_or_insert_str(&mut layouts.strings, s);
            Key::Character(static_str)
          } else {
            Key::Unidentified(NativeKeyCode::Unidentified)
          }
        }
        key => key,
      }
    };

    PartialKeyEventInfo {
      vkey,
      scancode,
      key_state: state,
      logical_key,
      key_without_modifiers,
      is_repeat: lparam_struct.is_repeat,
      code,
      location,
      utf16parts: Vec::with_capacity(8),
      text: PartialText::System(Vec::new()),
    }
  }

  fn finalize(self, strings: &mut HashSet<&'static str>) -> KeyEvent {
    let mut char_with_all_modifiers = None;
    if !self.utf16parts.is_empty() {
      let os_string = OsString::from_wide(&self.utf16parts);
      if let Ok(string) = os_string.into_string() {
        let static_str = get_or_insert_str(strings, string);
        char_with_all_modifiers = Some(static_str);
      }
    }

    // The text without Ctrl
    let mut text = None;
    match self.text {
      PartialText::System(wide) => {
        if !wide.is_empty() {
          let os_string = OsString::from_wide(&wide);
          if let Ok(string) = os_string.into_string() {
            let static_str = get_or_insert_str(strings, string);
            text = Some(static_str);
          }
        }
      }
      PartialText::Text(s) => {
        text = s;
      }
    }

    let logical_key = match self.logical_key {
      PartialLogicalKey::TextOr(fallback) => match text {
        Some(s) => {
          if s.grapheme_indices(true).count() > 1 {
            fallback
          } else {
            Key::Character(s)
          }
        }
        None => Key::Unidentified(NativeKeyCode::Windows(self.scancode)),
      },
      PartialLogicalKey::This(v) => v,
    };

    KeyEvent {
      physical_key: self.code,
      logical_key,
      text,
      location: self.location,
      state: self.key_state,
      repeat: self.is_repeat,
      platform_specific: KeyEventExtra {
        text_with_all_modifiers: char_with_all_modifiers,
        key_without_modifiers: self.key_without_modifiers,
      },
    }
  }
}

#[derive(Debug, Copy, Clone)]
struct KeyLParam {
  pub scancode: u8,
  pub extended: bool,

  /// This is `previous_state XOR transition_state`. See the lParam for WM_KEYDOWN and WM_KEYUP for further details.
  pub is_repeat: bool,
}

fn destructure_key_lparam(lparam: LPARAM) -> KeyLParam {
  let previous_state = (lparam.0 >> 30) & 0x01;
  let transition_state = (lparam.0 >> 31) & 0x01;
  KeyLParam {
    scancode: ((lparam.0 >> 16) & 0xFF) as u8,
    extended: ((lparam.0 >> 24) & 0x01) != 0,
    is_repeat: (previous_state ^ transition_state) != 0,
  }
}

#[inline]
fn new_ex_scancode(scancode: u8, extended: bool) -> ExScancode {
  (scancode as u16) | (if extended { 0xE000 } else { 0 })
}

#[inline]
fn ex_scancode_from_lparam(lparam: LPARAM) -> ExScancode {
  let lparam = destructure_key_lparam(lparam);
  new_ex_scancode(lparam.scancode, lparam.extended)
}

/// Gets the keyboard state as reported by messages that have been removed from the event queue.
/// See also: get_async_kbd_state
fn get_kbd_state() -> [u8; 256] {
  unsafe {
    let mut kbd_state: MaybeUninit<[u8; 256]> = MaybeUninit::uninit();
    GetKeyboardState(kbd_state.as_mut_ptr() as *mut u8);
    kbd_state.assume_init()
  }
}

/// Gets the current keyboard state regardless of whether the corresponding keyboard events have
/// been removed from the event queue. See also: get_kbd_state
fn get_async_kbd_state() -> [u8; 256] {
  unsafe {
    let mut kbd_state: [u8; 256] = [0; 256];
    for (vk, state) in kbd_state.iter_mut().enumerate() {
      let vk = vk as VIRTUAL_KEY;
      let async_state = GetAsyncKeyState(i32::from(vk));
      let is_down = (async_state & (1 << 15)) != 0;
      if is_down {
        *state = 0x80;
      }

      if matches!(
        vk,
        win32km::VK_CAPITAL | win32km::VK_NUMLOCK | win32km::VK_SCROLL
      ) {
        // Toggle states aren't reported by `GetAsyncKeyState`
        let toggle_state = GetKeyState(i32::from(vk));
        let is_active = (toggle_state & 1) != 0;
        *state |= if is_active { 1 } else { 0 };
      }
    }
    kbd_state
  }
}

/// On windows, AltGr == Ctrl + Alt
///
/// Due to this equivalence, the system generates a fake Ctrl key-press (and key-release) preceeding
/// every AltGr key-press (and key-release). We check if the current event is a Ctrl event and if
/// the next event is a right Alt (AltGr) event. If this is the case, the current event must be the
/// fake Ctrl event.
fn is_current_fake(curr_info: &PartialKeyEventInfo, next_msg: MSG, layout: &Layout) -> bool {
  let curr_is_ctrl = matches!(curr_info.logical_key, PartialLogicalKey::This(Key::Control));
  if layout.has_alt_graph {
    let next_code = ex_scancode_from_lparam(next_msg.lParam);
    let next_is_altgr = next_code == 0xE038; // 0xE038 is right alt
    if curr_is_ctrl && next_is_altgr {
      return true;
    }
  }
  false
}

fn get_location(scancode: ExScancode, hkl: HKL) -> KeyLocation {
  const VK_ABNT_C2: VIRTUAL_KEY = win32km::VK_ABNT_C2 as VIRTUAL_KEY;

  let extension = 0xE000;
  let extended = (scancode & extension) == extension;
  let vkey = unsafe { MapVirtualKeyExW(scancode as u32, MAPVK_VSC_TO_VK_EX, hkl) as u16 };

  // Use the native VKEY and the extended flag to cover most cases
  // This is taken from the `druid` GUI library, specifically
  // druid-shell/src/platform/windows/keyboard.rs
  match vkey {
    win32km::VK_LSHIFT | win32km::VK_LCONTROL | win32km::VK_LMENU | win32km::VK_LWIN => {
      KeyLocation::Left
    }
    win32km::VK_RSHIFT | win32km::VK_RCONTROL | win32km::VK_RMENU | win32km::VK_RWIN => {
      KeyLocation::Right
    }
    win32km::VK_RETURN if extended => KeyLocation::Numpad,
    win32km::VK_INSERT
    | win32km::VK_DELETE
    | win32km::VK_END
    | win32km::VK_DOWN
    | win32km::VK_NEXT
    | win32km::VK_LEFT
    | win32km::VK_CLEAR
    | win32km::VK_RIGHT
    | win32km::VK_HOME
    | win32km::VK_UP
    | win32km::VK_PRIOR => {
      if extended {
        KeyLocation::Standard
      } else {
        KeyLocation::Numpad
      }
    }
    win32km::VK_NUMPAD0
    | win32km::VK_NUMPAD1
    | win32km::VK_NUMPAD2
    | win32km::VK_NUMPAD3
    | win32km::VK_NUMPAD4
    | win32km::VK_NUMPAD5
    | win32km::VK_NUMPAD6
    | win32km::VK_NUMPAD7
    | win32km::VK_NUMPAD8
    | win32km::VK_NUMPAD9
    | win32km::VK_DECIMAL
    | win32km::VK_DIVIDE
    | win32km::VK_MULTIPLY
    | win32km::VK_SUBTRACT
    | win32km::VK_ADD
    | VK_ABNT_C2 => KeyLocation::Numpad,
    _ => KeyLocation::Standard,
  }
}

// used to build accelerators table from Key
pub(crate) fn key_to_vk(key: &KeyCode) -> Option<VIRTUAL_KEY> {
  Some(match key {
    KeyCode::KeyA => unsafe { VkKeyScanW('a' as u16) as VIRTUAL_KEY },
    KeyCode::KeyB => unsafe { VkKeyScanW('b' as u16) as VIRTUAL_KEY },
    KeyCode::KeyC => unsafe { VkKeyScanW('c' as u16) as VIRTUAL_KEY },
    KeyCode::KeyD => unsafe { VkKeyScanW('d' as u16) as VIRTUAL_KEY },
    KeyCode::KeyE => unsafe { VkKeyScanW('e' as u16) as VIRTUAL_KEY },
    KeyCode::KeyF => unsafe { VkKeyScanW('f' as u16) as VIRTUAL_KEY },
    KeyCode::KeyG => unsafe { VkKeyScanW('g' as u16) as VIRTUAL_KEY },
    KeyCode::KeyH => unsafe { VkKeyScanW('h' as u16) as VIRTUAL_KEY },
    KeyCode::KeyI => unsafe { VkKeyScanW('i' as u16) as VIRTUAL_KEY },
    KeyCode::KeyJ => unsafe { VkKeyScanW('j' as u16) as VIRTUAL_KEY },
    KeyCode::KeyK => unsafe { VkKeyScanW('k' as u16) as VIRTUAL_KEY },
    KeyCode::KeyL => unsafe { VkKeyScanW('l' as u16) as VIRTUAL_KEY },
    KeyCode::KeyM => unsafe { VkKeyScanW('m' as u16) as VIRTUAL_KEY },
    KeyCode::KeyN => unsafe { VkKeyScanW('n' as u16) as VIRTUAL_KEY },
    KeyCode::KeyO => unsafe { VkKeyScanW('o' as u16) as VIRTUAL_KEY },
    KeyCode::KeyP => unsafe { VkKeyScanW('p' as u16) as VIRTUAL_KEY },
    KeyCode::KeyQ => unsafe { VkKeyScanW('q' as u16) as VIRTUAL_KEY },
    KeyCode::KeyR => unsafe { VkKeyScanW('r' as u16) as VIRTUAL_KEY },
    KeyCode::KeyS => unsafe { VkKeyScanW('s' as u16) as VIRTUAL_KEY },
    KeyCode::KeyT => unsafe { VkKeyScanW('t' as u16) as VIRTUAL_KEY },
    KeyCode::KeyU => unsafe { VkKeyScanW('u' as u16) as VIRTUAL_KEY },
    KeyCode::KeyV => unsafe { VkKeyScanW('v' as u16) as VIRTUAL_KEY },
    KeyCode::KeyW => unsafe { VkKeyScanW('w' as u16) as VIRTUAL_KEY },
    KeyCode::KeyX => unsafe { VkKeyScanW('x' as u16) as VIRTUAL_KEY },
    KeyCode::KeyY => unsafe { VkKeyScanW('y' as u16) as VIRTUAL_KEY },
    KeyCode::KeyZ => unsafe { VkKeyScanW('z' as u16) as VIRTUAL_KEY },
    KeyCode::Digit0 => unsafe { VkKeyScanW('0' as u16) as VIRTUAL_KEY },
    KeyCode::Digit1 => unsafe { VkKeyScanW('1' as u16) as VIRTUAL_KEY },
    KeyCode::Digit2 => unsafe { VkKeyScanW('2' as u16) as VIRTUAL_KEY },
    KeyCode::Digit3 => unsafe { VkKeyScanW('3' as u16) as VIRTUAL_KEY },
    KeyCode::Digit4 => unsafe { VkKeyScanW('4' as u16) as VIRTUAL_KEY },
    KeyCode::Digit5 => unsafe { VkKeyScanW('5' as u16) as VIRTUAL_KEY },
    KeyCode::Digit6 => unsafe { VkKeyScanW('6' as u16) as VIRTUAL_KEY },
    KeyCode::Digit7 => unsafe { VkKeyScanW('7' as u16) as VIRTUAL_KEY },
    KeyCode::Digit8 => unsafe { VkKeyScanW('8' as u16) as VIRTUAL_KEY },
    KeyCode::Digit9 => unsafe { VkKeyScanW('9' as u16) as VIRTUAL_KEY },
    KeyCode::Comma => VK_OEM_COMMA,
    KeyCode::Minus => VK_OEM_MINUS,
    KeyCode::Period => VK_OEM_PERIOD,
    KeyCode::Equal => unsafe { VkKeyScanW('=' as u16) as VIRTUAL_KEY },
    KeyCode::Semicolon => unsafe { VkKeyScanW(';' as u16) as VIRTUAL_KEY },
    KeyCode::Slash => unsafe { VkKeyScanW('/' as u16) as VIRTUAL_KEY },
    KeyCode::Backslash => unsafe { VkKeyScanW('\\' as u16) as VIRTUAL_KEY },
    KeyCode::Quote => unsafe { VkKeyScanW('\'' as u16) as VIRTUAL_KEY },
    KeyCode::Backquote => unsafe { VkKeyScanW('`' as u16) as VIRTUAL_KEY },
    KeyCode::BracketLeft => unsafe { VkKeyScanW('[' as u16) as VIRTUAL_KEY },
    KeyCode::BracketRight => unsafe { VkKeyScanW(']' as u16) as VIRTUAL_KEY },
    KeyCode::Backspace => VK_BACK,
    KeyCode::Tab => VK_TAB,
    KeyCode::Space => VK_SPACE,
    KeyCode::Enter => VK_RETURN,
    KeyCode::Pause => VK_PAUSE,
    KeyCode::CapsLock => VK_CAPITAL,
    KeyCode::KanaMode => VK_KANA,
    KeyCode::Escape => VK_ESCAPE,
    KeyCode::NonConvert => VK_NONCONVERT,
    KeyCode::PageUp => VK_PRIOR,
    KeyCode::PageDown => VK_NEXT,
    KeyCode::End => VK_END,
    KeyCode::Home => VK_HOME,
    KeyCode::ArrowLeft => VK_LEFT,
    KeyCode::ArrowUp => VK_UP,
    KeyCode::ArrowRight => VK_RIGHT,
    KeyCode::ArrowDown => VK_DOWN,
    KeyCode::PrintScreen => VK_SNAPSHOT,
    KeyCode::Insert => VK_INSERT,
    KeyCode::Delete => VK_DELETE,
    KeyCode::Help => VK_HELP,
    KeyCode::ContextMenu => VK_APPS,
    KeyCode::F1 => VK_F1,
    KeyCode::F2 => VK_F2,
    KeyCode::F3 => VK_F3,
    KeyCode::F4 => VK_F4,
    KeyCode::F5 => VK_F5,
    KeyCode::F6 => VK_F6,
    KeyCode::F7 => VK_F7,
    KeyCode::F8 => VK_F8,
    KeyCode::F9 => VK_F9,
    KeyCode::F10 => VK_F10,
    KeyCode::F11 => VK_F11,
    KeyCode::F12 => VK_F12,
    KeyCode::F13 => VK_F13,
    KeyCode::F14 => VK_F14,
    KeyCode::F15 => VK_F15,
    KeyCode::F16 => VK_F16,
    KeyCode::F17 => VK_F17,
    KeyCode::F18 => VK_F18,
    KeyCode::F19 => VK_F19,
    KeyCode::F20 => VK_F20,
    KeyCode::F21 => VK_F21,
    KeyCode::F22 => VK_F22,
    KeyCode::F23 => VK_F23,
    KeyCode::F24 => VK_F24,
    KeyCode::NumLock => VK_NUMLOCK,
    KeyCode::ScrollLock => VK_SCROLL,
    KeyCode::BrowserBack => VK_BROWSER_BACK,
    KeyCode::BrowserForward => VK_BROWSER_FORWARD,
    KeyCode::BrowserRefresh => VK_BROWSER_REFRESH,
    KeyCode::BrowserStop => VK_BROWSER_STOP,
    KeyCode::BrowserSearch => VK_BROWSER_SEARCH,
    KeyCode::BrowserFavorites => VK_BROWSER_FAVORITES,
    KeyCode::BrowserHome => VK_BROWSER_HOME,
    KeyCode::AudioVolumeMute => VK_VOLUME_MUTE,
    KeyCode::AudioVolumeDown => VK_VOLUME_DOWN,
    KeyCode::AudioVolumeUp => VK_VOLUME_UP,
    KeyCode::MediaTrackNext => VK_MEDIA_NEXT_TRACK,
    KeyCode::MediaTrackPrevious => VK_MEDIA_PREV_TRACK,
    KeyCode::MediaStop => VK_MEDIA_STOP,
    KeyCode::MediaPlayPause => VK_MEDIA_PLAY_PAUSE,
    KeyCode::LaunchMail => VK_LAUNCH_MAIL,
    KeyCode::Convert => VK_CONVERT,
    _ => return None,
  })
}
