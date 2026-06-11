use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use windows::Win32::{
  Foundation::{HWND, LPARAM, LRESULT, WPARAM},
  UI::WindowsAndMessaging as win32wm,
};

use crate::platform_impl::platform::{event_loop::ProcResult, keyboard::next_kbd_msg};

pub fn is_msg_ime_related(msg_kind: u32) -> bool {
  matches!(
    msg_kind,
    win32wm::WM_IME_COMPOSITION
      | win32wm::WM_IME_COMPOSITIONFULL
      | win32wm::WM_IME_STARTCOMPOSITION
      | win32wm::WM_IME_ENDCOMPOSITION
      | win32wm::WM_IME_CHAR
      | win32wm::WM_CHAR
      | win32wm::WM_SYSCHAR
  )
}

pub struct MinimalIme {
  // True if we're currently receiving messages belonging to a finished IME session.
  getting_ime_text: AtomicBool,

  utf16parts: Mutex<Vec<u16>>,
}
impl Default for MinimalIme {
  fn default() -> Self {
    MinimalIme {
      getting_ime_text: AtomicBool::new(false),
      utf16parts: Mutex::new(Vec::with_capacity(16)),
    }
  }
}
impl MinimalIme {
  pub(crate) fn process_message(
    &self,
    hwnd: HWND,
    msg_kind: u32,
    wparam: WPARAM,
    _lparam: LPARAM,
    result: &mut ProcResult,
  ) -> Option<String> {
    match msg_kind {
      win32wm::WM_IME_ENDCOMPOSITION => {
        self.getting_ime_text.store(true, Ordering::Relaxed);
      }
      win32wm::WM_CHAR | win32wm::WM_SYSCHAR if self.getting_ime_text.load(Ordering::Relaxed) => {
        *result = ProcResult::Value(LRESULT(0));
        self.utf16parts.lock().push(wparam.0 as u16);
        // It's important that we push the new character and release the lock
        // before getting the next message
        let next_msg = next_kbd_msg(hwnd);
        let more_char_coming = next_msg
          .map(|m| matches!(m.message, win32wm::WM_CHAR | win32wm::WM_SYSCHAR))
          .unwrap_or(false);
        if !more_char_coming {
          let mut utf16parts = self.utf16parts.lock();
          let result = String::from_utf16(&utf16parts).ok();
          utf16parts.clear();
          self.getting_ime_text.store(false, Ordering::Relaxed);
          return result;
        }
      }
      _ => (),
    }

    None
  }
}
