use super::window::{WindowId, WindowRequest};
use crate::{
  accelerator::{Accelerator, AcceleratorId},
  event_loop::EventLoopWindowTarget,
  global_shortcut::{GlobalShortcut as RootGlobalShortcut, ShortcutManagerError},
  keyboard::KeyCode,
};
use crossbeam_channel::{self as channel, Receiver, Sender, TryRecvError};
use std::{
  collections::HashMap,
  ptr,
  sync::{Arc, Mutex},
};
use x11_dl::{keysym, xlib};

#[derive(Debug)]
enum HotkeyMessage {
  RegisterHotkey(ListenerId, u32, u32),
  RegisterHotkeyResult(Result<ListenerId, ShortcutManagerError>),
  UnregisterHotkey(ListenerId),
  UnregisterHotkeyResult(Result<(), ShortcutManagerError>),
  DropThread,
}

#[derive(Debug)]
pub struct ShortcutManager {
  shortcuts: ListenerMap,
  method_sender: Sender<HotkeyMessage>,
  method_receiver: Receiver<HotkeyMessage>,
}

impl ShortcutManager {
  pub(crate) fn new<T>(_window_target: &EventLoopWindowTarget<T>) -> Self {
    let window_id = WindowId::dummy();
    let hotkeys = ListenerMap::default();
    let hotkey_map = hotkeys.clone();

    let event_loop_channel = _window_target.p.window_requests_tx.clone();

    let (method_sender, thread_receiver) = channel::unbounded();
    let (thread_sender, method_receiver) = channel::unbounded();

    std::thread::spawn(move || {
      let event_loop_channel = event_loop_channel.clone();
      let xlib = xlib::Xlib::open().unwrap();
      unsafe {
        let display = (xlib.XOpenDisplay)(ptr::null());
        let root = (xlib.XDefaultRootWindow)(display);

        // Only trigger key release at end of repeated keys
        #[allow(clippy::uninit_assumed_init)]
        let mut supported_rtrn: i32 = std::mem::MaybeUninit::uninit().assume_init();
        (xlib.XkbSetDetectableAutoRepeat)(display, 1, &mut supported_rtrn);

        (xlib.XSelectInput)(display, root, xlib::KeyReleaseMask);
        #[allow(clippy::uninit_assumed_init)]
        let mut event: xlib::XEvent = std::mem::MaybeUninit::uninit().assume_init();

        loop {
          let event_loop_channel = event_loop_channel.clone();
          if (xlib.XPending)(display) > 0 {
            (xlib.XNextEvent)(display, &mut event);
            if let xlib::KeyRelease = event.get_type() {
              let keycode = event.key.keycode;
              let modifiers = event.key.state;
              if let Some(hotkey_id) = hotkey_map.lock().unwrap().get(&(keycode as i32, modifiers))
              {
                event_loop_channel
                  .send((window_id, WindowRequest::GlobalHotKey(*hotkey_id as u16)))
                  .unwrap();
              }
            }
          }

          match thread_receiver.try_recv() {
            Ok(HotkeyMessage::RegisterHotkey(_, modifiers, key)) => {
              let keycode = (xlib.XKeysymToKeycode)(display, key.into()) as i32;

              let result = (xlib.XGrabKey)(
                display,
                keycode,
                modifiers,
                root,
                0,
                xlib::GrabModeAsync,
                xlib::GrabModeAsync,
              );
              if result == 0 {
                if let Err(err) = thread_sender
                  .clone()
                  .send(HotkeyMessage::RegisterHotkeyResult(Err(
                    ShortcutManagerError::InvalidAccelerator(
                      "Unable to register accelerator".into(),
                    ),
                  )))
                {
                  #[cfg(debug_assertions)]
                  eprintln!("hotkey: thread_sender.send error {}", err);
                }
              } else if let Err(err) = thread_sender.send(HotkeyMessage::RegisterHotkeyResult(Ok(
                (keycode, modifiers),
              ))) {
                #[cfg(debug_assertions)]
                eprintln!("hotkey: thread_sender.send error {}", err);
              }
            }
            Ok(HotkeyMessage::UnregisterHotkey(id)) => {
              let result = (xlib.XUngrabKey)(display, id.0, id.1, root);
              if result == 0 {
                if let Err(err) = thread_sender
                  .clone()
                  .send(HotkeyMessage::UnregisterHotkeyResult(Err(
                    ShortcutManagerError::InvalidAccelerator(
                      "Unable to unregister accelerator".into(),
                    ),
                  )))
                {
                  #[cfg(debug_assertions)]
                  eprintln!("hotkey: thread_sender.send error {}", err);
                }
              } else if let Err(err) =
                thread_sender.send(HotkeyMessage::UnregisterHotkeyResult(Ok(())))
              {
                #[cfg(debug_assertions)]
                eprintln!("hotkey: thread_sender.send error {}", err);
              }
            }
            Ok(HotkeyMessage::DropThread) => {
              (xlib.XCloseDisplay)(display);
              return;
            }
            Err(err) => {
              if let TryRecvError::Disconnected = err {
                #[cfg(debug_assertions)]
                eprintln!("hotkey: try_recv error {}", err);
              }
            }
            _ => unreachable!("other message should not arrive"),
          };

          std::thread::sleep(std::time::Duration::from_millis(50));
        }
      }
    });

    ShortcutManager {
      shortcuts: hotkeys,
      method_sender,
      method_receiver,
    }
  }

  pub(crate) fn register(
    &mut self,
    accelerator: Accelerator,
  ) -> Result<RootGlobalShortcut, ShortcutManagerError> {
    let keycode = get_x11_scancode_from_hotkey(accelerator.key);

    if let Some(keycode) = keycode {
      let mut converted_modifiers: u32 = 0;
      if accelerator.mods.shift_key() {
        converted_modifiers |= xlib::ShiftMask;
      }
      if accelerator.mods.super_key() {
        converted_modifiers |= xlib::Mod4Mask;
      }
      if accelerator.mods.alt_key() {
        converted_modifiers |= xlib::Mod1Mask;
      }
      if accelerator.mods.control_key() {
        converted_modifiers |= xlib::ControlMask;
      }

      self
        .method_sender
        .send(HotkeyMessage::RegisterHotkey(
          (0, 0),
          converted_modifiers,
          keycode,
        ))
        .map_err(|_| {
          ShortcutManagerError::InvalidAccelerator("Unable to register global shortcut".into())
        })?;

      return match self.method_receiver.recv() {
        Ok(HotkeyMessage::RegisterHotkeyResult(Ok(id))) => {
          self
            .shortcuts
            .lock()
            .unwrap()
            .insert(id, accelerator.clone().id().0 as u32);
          let shortcut = GlobalShortcut { accelerator };
          return Ok(RootGlobalShortcut(shortcut));
        }
        Ok(HotkeyMessage::RegisterHotkeyResult(Err(err))) => Err(err),
        Err(err) => Err(ShortcutManagerError::InvalidAccelerator(err.to_string())),
        _ => Err(ShortcutManagerError::InvalidAccelerator(
          "Unknown error".into(),
        )),
      };
    }

    Err(ShortcutManagerError::InvalidAccelerator(
      "Invalid accelerators".into(),
    ))
  }

  pub(crate) fn unregister_all(&mut self) -> Result<(), ShortcutManagerError> {
    for (found_id, _) in self.shortcuts.lock().unwrap().iter() {
      self
        .method_sender
        .send(HotkeyMessage::UnregisterHotkey(*found_id))
        .map_err(|_| ShortcutManagerError::InvalidAccelerator("Channel error".into()))?;
    }
    self.shortcuts = ListenerMap::default();
    Ok(())
  }

  pub(crate) fn unregister(
    &self,
    shortcut: RootGlobalShortcut,
  ) -> Result<(), ShortcutManagerError> {
    let mut found_id = (-1, 0);
    for (id, shortcut_id) in self.shortcuts.lock().unwrap().iter() {
      if *shortcut_id == shortcut.0.id().0 as u32 {
        found_id = *id;
        break;
      }
    }
    if found_id == (-1, 0) {
      return Err(ShortcutManagerError::AcceleratorNotRegistered(
        shortcut.0.accelerator,
      ));
    }

    self
      .method_sender
      .send(HotkeyMessage::UnregisterHotkey(found_id))
      .map_err(|_| ShortcutManagerError::InvalidAccelerator("Channel error".into()))?;
    if self.shortcuts.lock().unwrap().remove(&found_id).is_none() {
      panic!("hotkey should never be none")
    };
    match self.method_receiver.recv() {
      Ok(HotkeyMessage::UnregisterHotkeyResult(Ok(_))) => Ok(()),
      Ok(HotkeyMessage::UnregisterHotkeyResult(Err(err))) => Err(err),
      Err(err) => Err(ShortcutManagerError::InvalidAccelerator(err.to_string())),
      _ => Err(ShortcutManagerError::InvalidAccelerator(
        "Unknown error".into(),
      )),
    }
  }
}

impl Drop for ShortcutManager {
  fn drop(&mut self) {
    if let Err(err) = self.method_sender.send(HotkeyMessage::DropThread) {
      #[cfg(debug_assertions)]
      eprintln!("cant send close thread message {}", err);
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalShortcut {
  pub(crate) accelerator: Accelerator,
}
type ListenerId = (i32, u32);
type ListenerMap = Arc<Mutex<HashMap<ListenerId, u32>>>;

impl GlobalShortcut {
  pub fn id(&self) -> AcceleratorId {
    self.accelerator.clone().id()
  }
}

// required for event but we use dummy window Id
// so it shouldn't be a problem
unsafe impl Send for WindowId {}
unsafe impl Sync for WindowId {}
// simple enum, no pointer, shouldn't be a problem
// to use send + sync
unsafe impl Send for WindowRequest {}
unsafe impl Sync for WindowRequest {}

fn get_x11_scancode_from_hotkey(key: KeyCode) -> Option<u32> {
  Some(match key {
    KeyCode::KeyA => 'A' as u32,
    KeyCode::KeyB => 'B' as u32,
    KeyCode::KeyC => 'C' as u32,
    KeyCode::KeyD => 'D' as u32,
    KeyCode::KeyE => 'E' as u32,
    KeyCode::KeyF => 'F' as u32,
    KeyCode::KeyG => 'G' as u32,
    KeyCode::KeyH => 'H' as u32,
    KeyCode::KeyI => 'I' as u32,
    KeyCode::KeyJ => 'J' as u32,
    KeyCode::KeyK => 'K' as u32,
    KeyCode::KeyL => 'L' as u32,
    KeyCode::KeyM => 'M' as u32,
    KeyCode::KeyN => 'N' as u32,
    KeyCode::KeyO => 'O' as u32,
    KeyCode::KeyP => 'P' as u32,
    KeyCode::KeyQ => 'Q' as u32,
    KeyCode::KeyR => 'R' as u32,
    KeyCode::KeyS => 'S' as u32,
    KeyCode::KeyT => 'T' as u32,
    KeyCode::KeyU => 'U' as u32,
    KeyCode::KeyV => 'V' as u32,
    KeyCode::KeyW => 'W' as u32,
    KeyCode::KeyX => 'X' as u32,
    KeyCode::KeyY => 'Y' as u32,
    KeyCode::KeyZ => 'Z' as u32,
    KeyCode::Backslash => keysym::XK_backslash,
    KeyCode::BracketLeft => keysym::XK_bracketleft,
    KeyCode::BracketRight => keysym::XK_bracketright,
    KeyCode::Comma => keysym::XK_comma,
    KeyCode::Digit0 => '0' as u32,
    KeyCode::Digit1 => '1' as u32,
    KeyCode::Digit2 => '2' as u32,
    KeyCode::Digit3 => '3' as u32,
    KeyCode::Digit4 => '4' as u32,
    KeyCode::Digit5 => '5' as u32,
    KeyCode::Digit6 => '6' as u32,
    KeyCode::Digit7 => '7' as u32,
    KeyCode::Digit8 => '8' as u32,
    KeyCode::Digit9 => '9' as u32,
    KeyCode::Equal => keysym::XK_equal,
    KeyCode::IntlBackslash => keysym::XK_backslash,
    KeyCode::Minus => keysym::XK_minus,
    KeyCode::Period => keysym::XK_period,
    KeyCode::Quote => keysym::XK_leftsinglequotemark,
    KeyCode::Semicolon => keysym::XK_semicolon,
    KeyCode::Slash => keysym::XK_slash,
    KeyCode::Backspace => keysym::XK_BackSpace,
    KeyCode::CapsLock => keysym::XK_Caps_Lock,
    KeyCode::Enter => keysym::XK_Return,
    KeyCode::Space => keysym::XK_space,
    KeyCode::Tab => keysym::XK_Tab,
    KeyCode::Delete => keysym::XK_Delete,
    KeyCode::End => keysym::XK_End,
    KeyCode::Home => keysym::XK_Home,
    KeyCode::Insert => keysym::XK_Insert,
    KeyCode::PageDown => keysym::XK_Page_Down,
    KeyCode::PageUp => keysym::XK_Page_Up,
    KeyCode::ArrowDown => keysym::XK_Down,
    KeyCode::ArrowLeft => keysym::XK_Left,
    KeyCode::ArrowRight => keysym::XK_Right,
    KeyCode::ArrowUp => keysym::XK_Up,
    KeyCode::Numpad0 => keysym::XK_KP_0,
    KeyCode::Numpad1 => keysym::XK_KP_1,
    KeyCode::Numpad2 => keysym::XK_KP_2,
    KeyCode::Numpad3 => keysym::XK_KP_3,
    KeyCode::Numpad4 => keysym::XK_KP_4,
    KeyCode::Numpad5 => keysym::XK_KP_5,
    KeyCode::Numpad6 => keysym::XK_KP_6,
    KeyCode::Numpad7 => keysym::XK_KP_7,
    KeyCode::Numpad8 => keysym::XK_KP_8,
    KeyCode::Numpad9 => keysym::XK_KP_9,
    KeyCode::NumpadAdd => keysym::XK_KP_Add,
    KeyCode::NumpadDecimal => keysym::XK_KP_Decimal,
    KeyCode::NumpadDivide => keysym::XK_KP_Divide,
    KeyCode::NumpadMultiply => keysym::XK_KP_Multiply,
    KeyCode::NumpadSubtract => keysym::XK_KP_Subtract,
    KeyCode::Escape => keysym::XK_Escape,
    KeyCode::PrintScreen => keysym::XK_Print,
    KeyCode::ScrollLock => keysym::XK_Scroll_Lock,
    KeyCode::Pause => keysym::XF86XK_AudioPlay,
    KeyCode::MediaStop => keysym::XF86XK_AudioStop,
    KeyCode::MediaTrackNext => keysym::XF86XK_AudioNext,
    KeyCode::MediaTrackPrevious => keysym::XF86XK_AudioPrev,
    KeyCode::AudioVolumeDown => keysym::XF86XK_AudioLowerVolume,
    KeyCode::AudioVolumeMute => keysym::XF86XK_AudioMute,
    KeyCode::AudioVolumeUp => keysym::XF86XK_AudioRaiseVolume,
    KeyCode::F1 => keysym::XK_F1,
    KeyCode::F2 => keysym::XK_F2,
    KeyCode::F3 => keysym::XK_F3,
    KeyCode::F4 => keysym::XK_F4,
    KeyCode::F5 => keysym::XK_F5,
    KeyCode::F6 => keysym::XK_F6,
    KeyCode::F7 => keysym::XK_F7,
    KeyCode::F8 => keysym::XK_F8,
    KeyCode::F9 => keysym::XK_F9,
    KeyCode::F10 => keysym::XK_F10,
    KeyCode::F11 => keysym::XK_F11,
    KeyCode::F12 => keysym::XK_F12,

    _ => return None,
  })
}
