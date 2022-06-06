//! **UNSTABLE** -- Types related to the keyboard.

// This file contains a substantial portion of the UI Events Specification by the W3C. In
// particular, the variant names within `Key` and `KeyCode` and their documentation are modified
// versions of contents of the aforementioned specification.
//
// The original documents are:
//
// ### For `Key`
// UI Events KeyboardEvent key Values
// https://www.w3.org/TR/2017/CR-uievents-key-20170601/
// Copyright © 2017 W3C® (MIT, ERCIM, Keio, Beihang).
//
// ### For `KeyCode`
// UI Events KeyboardEvent code Values
// https://www.w3.org/TR/2017/CR-uievents-code-20170601/
// Copyright © 2017 W3C® (MIT, ERCIM, Keio, Beihang).
//
// These documents were used under the terms of the following license. This W3C license as well as
// the W3C short notice apply to the `Key` and `KeyCode` enums and their variants and the
// documentation attached to their variants.

// --------- BEGGINING OF W3C LICENSE --------------------------------------------------------------
//
// License
//
// By obtaining and/or copying this work, you (the licensee) agree that you have read, understood,
// and will comply with the following terms and conditions.
//
// Permission to copy, modify, and distribute this work, with or without modification, for any
// purpose and without fee or royalty is hereby granted, provided that you include the following on
// ALL copies of the work or portions thereof, including modifications:
//
// - The full text of this NOTICE in a location viewable to users of the redistributed or derivative
//   work.
// - Any pre-existing intellectual property disclaimers, notices, or terms and conditions. If none
//   exist, the W3C Software and Document Short Notice should be included.
// - Notice of any changes or modifications, through a copyright statement on the new code or
//   document such as "This software or document includes material copied from or derived from
//   [title and URI of the W3C document]. Copyright © [YEAR] W3C® (MIT, ERCIM, Keio, Beihang)."
//
// Disclaimers
//
// THIS WORK IS PROVIDED "AS IS," AND COPYRIGHT HOLDERS MAKE NO REPRESENTATIONS OR WARRANTIES,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO, WARRANTIES OF MERCHANTABILITY OR FITNESS FOR
// ANY PARTICULAR PURPOSE OR THAT THE USE OF THE SOFTWARE OR DOCUMENT WILL NOT INFRINGE ANY THIRD
// PARTY PATENTS, COPYRIGHTS, TRADEMARKS OR OTHER RIGHTS.
//
// COPYRIGHT HOLDERS WILL NOT BE LIABLE FOR ANY DIRECT, INDIRECT, SPECIAL OR CONSEQUENTIAL DAMAGES
// ARISING OUT OF ANY USE OF THE SOFTWARE OR DOCUMENT.
//
// The name and trademarks of copyright holders may NOT be used in advertising or publicity
// pertaining to the work without specific, written prior permission. Title to copyright in this
// work will at all times remain with copyright holders.
//
// --------- END OF W3C LICENSE --------------------------------------------------------------------

// --------- BEGGINING OF W3C SHORT NOTICE ---------------------------------------------------------
//
// tao: https://github.com/tauri-apps/tao
//
// Copyright © 2021 World Wide Web Consortium, (Massachusetts Institute of Technology, European
// Research Consortium for Informatics and Mathematics, Keio University, Beihang). All Rights
// Reserved. This work is distributed under the W3C® Software License [1] in the hope that it will
// be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE.
//
// [1] http://www.w3.org/Consortium/Legal/copyright-software
//
// --------- END OF W3C SHORT NOTICE ---------------------------------------------------------------

use std::{fmt, str::FromStr};

use crate::{
  error::OsError,
  platform_impl::{
    keycode_from_scancode as platform_keycode_from_scancode,
    keycode_to_scancode as platform_keycode_to_scancode,
  },
};

impl ModifiersState {
  /// Returns `true` if the shift key is pressed.
  pub fn shift(&self) -> bool {
    self.intersects(Self::SHIFT)
  }
  /// Returns `true` if the control key is pressed.
  pub fn ctrl(&self) -> bool {
    self.intersects(Self::CONTROL)
  }
  /// Returns `true` if the alt key is pressed.
  pub fn alt(&self) -> bool {
    self.intersects(Self::ALT)
  }
  /// Returns `true` if the super key is pressed.
  pub fn logo(&self) -> bool {
    self.intersects(Self::LOGO)
  }
}

bitflags! {
    /// Represents the current state of the keyboard modifiers
    ///
    /// Each flag represents a modifier and is set if this modifier is active.
    #[derive(Default)]
    pub struct ModifiersState: u32 {
        // left and right modifiers are currently commented out, but we should be able to support
        // them in a future release
        /// The "shift" key.
        const SHIFT = 0b100 << 0;
        // const LSHIFT = 0b010 << 0;
        // const RSHIFT = 0b001 << 0;
        /// The "control" key.
        const CONTROL = 0b100 << 3;
        // const LCTRL = 0b010 << 3;
        // const RCTRL = 0b001 << 3;
        /// The "alt" key.
        const ALT = 0b100 << 6;
        // const LALT = 0b010 << 6;
        // const RALT = 0b001 << 6;
        /// This is the "windows" key on PC and "command" key on Mac.
        const LOGO = 0b100 << 9;
        // const LSUPER  = 0b010 << 9;
        // const RSUPER  = 0b001 << 9;
    }
}

#[cfg(feature = "serde")]
mod modifiers_serde {
  use super::ModifiersState;
  use serde::{Deserialize, Deserializer, Serialize, Serializer};

  #[derive(Default, Serialize, Deserialize)]
  #[serde(default)]
  #[serde(rename = "ModifiersState")]
  pub struct ModifiersStateSerialize {
    pub shift_key: bool,
    pub control_key: bool,
    pub alt_key: bool,
    pub super_key: bool,
  }

  impl Serialize for ModifiersState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
      S: Serializer,
    {
      let s = ModifiersStateSerialize {
        shift_key: self.shift_key(),
        control_key: self.control_key(),
        alt_key: self.alt_key(),
        super_key: self.super_key(),
      };
      s.serialize(serializer)
    }
  }

  impl<'de> Deserialize<'de> for ModifiersState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
      D: Deserializer<'de>,
    {
      let ModifiersStateSerialize {
        shift_key,
        control_key,
        alt_key,
        super_key,
      } = ModifiersStateSerialize::deserialize(deserializer)?;
      let mut m = ModifiersState::empty();
      m.set(ModifiersState::SHIFT, shift_key);
      m.set(ModifiersState::CONTROL, control_key);
      m.set(ModifiersState::ALT, alt_key);
      m.set(ModifiersState::LOGO, super_key);
      Ok(m)
    }
  }
}

/// Contains the platform-native physical key identifier (aka scancode)
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NativeKeyCode {
  Unidentified,
  Windows(u16),
  MacOS(u16),
  Gtk(u16),

  /// This is the android "key code" of the event as returned by
  /// `KeyEvent.getKeyCode()`
  Android(u32),
}

/// Represents the code of a physical key.
///
/// This mostly conforms to the UI Events Specification's [`KeyboardEvent.code`] with a few
/// exceptions:
/// - The keys that the specification calls "MetaLeft" and "MetaRight" are named "SuperLeft" and
///   "SuperRight" here.
/// - The key that the specification calls "Super" is reported as `Unidentified` here.
/// - The `Unidentified` variant here, can still identifiy a key through it's `NativeKeyCode`.
///
/// [`KeyboardEvent.code`]: https://w3c.github.io/uievents-code/#code-value-tables
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum KeyCode {
  /// This variant is used when the key cannot be translated to any
  /// other variant.
  ///
  /// The native scancode is provided (if available) in order
  /// to allow the user to specify keybindings for keys which
  /// are not defined by this API.
  Unidentified(NativeKeyCode),
  /// <kbd>`</kbd> on a US keyboard. This is also called a backtick or grave.
  /// This is the <kbd>半角</kbd>/<kbd>全角</kbd>/<kbd>漢字</kbd>
  /// (hankaku/zenkaku/kanji) key on Japanese keyboards
  Backquote,
  /// Used for both the US <kbd>\\</kbd> (on the 101-key layout) and also for the key
  /// located between the <kbd>"</kbd> and <kbd>Enter</kbd> keys on row C of the 102-,
  /// 104- and 106-key layouts.
  /// Labeled <kbd>#</kbd> on a UK (102) keyboard.
  Backslash,
  /// <kbd>[</kbd> on a US keyboard.
  BracketLeft,
  /// <kbd>]</kbd> on a US keyboard.
  BracketRight,
  /// <kbd>,</kbd> on a US keyboard.
  Comma,
  /// <kbd>0</kbd> on a US keyboard.
  Digit0,
  /// <kbd>1</kbd> on a US keyboard.
  Digit1,
  /// <kbd>2</kbd> on a US keyboard.
  Digit2,
  /// <kbd>3</kbd> on a US keyboard.
  Digit3,
  /// <kbd>4</kbd> on a US keyboard.
  Digit4,
  /// <kbd>5</kbd> on a US keyboard.
  Digit5,
  /// <kbd>6</kbd> on a US keyboard.
  Digit6,
  /// <kbd>7</kbd> on a US keyboard.
  Digit7,
  /// <kbd>8</kbd> on a US keyboard.
  Digit8,
  /// <kbd>9</kbd> on a US keyboard.
  Digit9,
  /// <kbd>=</kbd> on a US keyboard.
  Equal,
  /// Located between the left <kbd>Shift</kbd> and <kbd>Z</kbd> keys.
  /// Labeled <kbd>\\</kbd> on a UK keyboard.
  IntlBackslash,
  /// Located between the <kbd>/</kbd> and right <kbd>Shift</kbd> keys.
  /// Labeled <kbd>\\</kbd> (ro) on a Japanese keyboard.
  IntlRo,
  /// Located between the <kbd>=</kbd> and <kbd>Backspace</kbd> keys.
  /// Labeled <kbd>¥</kbd> (yen) on a Japanese keyboard. <kbd>\\</kbd> on a
  /// Russian keyboard.
  IntlYen,
  /// <kbd>a</kbd> on a US keyboard.
  /// Labeled <kbd>q</kbd> on an AZERTY (e.g., French) keyboard.
  KeyA,
  /// <kbd>b</kbd> on a US keyboard.
  KeyB,
  /// <kbd>c</kbd> on a US keyboard.
  KeyC,
  /// <kbd>d</kbd> on a US keyboard.
  KeyD,
  /// <kbd>e</kbd> on a US keyboard.
  KeyE,
  /// <kbd>f</kbd> on a US keyboard.
  KeyF,
  /// <kbd>g</kbd> on a US keyboard.
  KeyG,
  /// <kbd>h</kbd> on a US keyboard.
  KeyH,
  /// <kbd>i</kbd> on a US keyboard.
  KeyI,
  /// <kbd>j</kbd> on a US keyboard.
  KeyJ,
  /// <kbd>k</kbd> on a US keyboard.
  KeyK,
  /// <kbd>l</kbd> on a US keyboard.
  KeyL,
  /// <kbd>m</kbd> on a US keyboard.
  KeyM,
  /// <kbd>n</kbd> on a US keyboard.
  KeyN,
  /// <kbd>o</kbd> on a US keyboard.
  KeyO,
  /// <kbd>p</kbd> on a US keyboard.
  KeyP,
  /// <kbd>q</kbd> on a US keyboard.
  /// Labeled <kbd>a</kbd> on an AZERTY (e.g., French) keyboard.
  KeyQ,
  /// <kbd>r</kbd> on a US keyboard.
  KeyR,
  /// <kbd>s</kbd> on a US keyboard.
  KeyS,
  /// <kbd>t</kbd> on a US keyboard.
  KeyT,
  /// <kbd>u</kbd> on a US keyboard.
  KeyU,
  /// <kbd>v</kbd> on a US keyboard.
  KeyV,
  /// <kbd>w</kbd> on a US keyboard.
  /// Labeled <kbd>z</kbd> on an AZERTY (e.g., French) keyboard.
  KeyW,
  /// <kbd>x</kbd> on a US keyboard.
  KeyX,
  /// <kbd>y</kbd> on a US keyboard.
  /// Labeled <kbd>z</kbd> on a QWERTZ (e.g., German) keyboard.
  KeyY,
  /// <kbd>z</kbd> on a US keyboard.
  /// Labeled <kbd>w</kbd> on an AZERTY (e.g., French) keyboard, and <kbd>y</kbd> on a
  /// QWERTZ (e.g., German) keyboard.
  KeyZ,
  /// <kbd>-</kbd> on a US keyboard.
  Minus,
  /// <kbd>.</kbd> on a US keyboard.
  Period,
  /// <kbd>'</kbd> on a US keyboard.
  Quote,
  /// <kbd>;</kbd> on a US keyboard.
  Semicolon,
  /// <kbd>/</kbd> on a US keyboard.
  Slash,
  /// <kbd>Alt</kbd>, <kbd>Option</kbd>, or <kbd>⌥</kbd>.
  AltLeft,
  /// <kbd>Alt</kbd>, <kbd>Option</kbd>, or <kbd>⌥</kbd>.
  /// This is labeled <kbd>AltGr</kbd> on many keyboard layouts.
  AltRight,
  /// <kbd>Backspace</kbd> or <kbd>⌫</kbd>.
  /// Labeled <kbd>Delete</kbd> on Apple keyboards.
  Backspace,
  /// <kbd>CapsLock</kbd> or <kbd>⇪</kbd>
  CapsLock,
  /// The application context menu key, which is typically found between the right
  /// <kbd>Super</kbd> key and the right <kbd>Control</kbd> key.
  ContextMenu,
  /// <kbd>Control</kbd> or <kbd>⌃</kbd>
  ControlLeft,
  /// <kbd>Control</kbd> or <kbd>⌃</kbd>
  ControlRight,
  /// <kbd>Enter</kbd> or <kbd>↵</kbd>. Labeled <kbd>Return</kbd> on Apple keyboards.
  Enter,
  /// The Windows, <kbd>⌘</kbd>, <kbd>Command</kbd>, or other OS symbol key.
  SuperLeft,
  /// The Windows, <kbd>⌘</kbd>, <kbd>Command</kbd>, or other OS symbol key.
  SuperRight,
  /// <kbd>Shift</kbd> or <kbd>⇧</kbd>
  ShiftLeft,
  /// <kbd>Shift</kbd> or <kbd>⇧</kbd>
  ShiftRight,
  /// <kbd> </kbd> (space)
  Space,
  /// <kbd>Tab</kbd> or <kbd>⇥</kbd>
  Tab,
  /// Japanese: <kbd>変</kbd> (henkan)
  Convert,
  /// Japanese: <kbd>カタカナ</kbd>/<kbd>ひらがな</kbd>/<kbd>ローマ字</kbd> (katakana/hiragana/romaji)
  KanaMode,
  /// Korean: HangulMode <kbd>한/영</kbd> (han/yeong)
  ///
  /// Japanese (Mac keyboard): <kbd>か</kbd> (kana)
  Lang1,
  /// Korean: Hanja <kbd>한</kbd> (hanja)
  ///
  /// Japanese (Mac keyboard): <kbd>英</kbd> (eisu)
  Lang2,
  /// Japanese (word-processing keyboard): Katakana
  Lang3,
  /// Japanese (word-processing keyboard): Hiragana
  Lang4,
  /// Japanese (word-processing keyboard): Zenkaku/Hankaku
  Lang5,
  /// Japanese: <kbd>無変換</kbd> (muhenkan)
  NonConvert,
  /// <kbd>⌦</kbd>. The forward delete key.
  /// Note that on Apple keyboards, the key labelled <kbd>Delete</kbd> on the main part of
  /// the keyboard is encoded as [`Backspace`].
  ///
  /// [`Backspace`]: Self::Backspace
  Delete,
  /// <kbd>Page Down</kbd>, <kbd>End</kbd>, or <kbd>↘</kbd>
  End,
  /// <kbd>Help</kbd>. Not present on standard PC keyboards.
  Help,
  /// <kbd>Home</kbd> or <kbd>↖</kbd>
  Home,
  /// <kbd>Insert</kbd> or <kbd>Ins</kbd>. Not present on Apple keyboards.
  Insert,
  /// <kbd>Page Down</kbd>, <kbd>PgDn</kbd>, or <kbd>⇟</kbd>
  PageDown,
  /// <kbd>Page Up</kbd>, <kbd>PgUp</kbd>, or <kbd>⇞</kbd>
  PageUp,
  /// <kbd>↓</kbd>
  ArrowDown,
  /// <kbd>←</kbd>
  ArrowLeft,
  /// <kbd>→</kbd>
  ArrowRight,
  /// <kbd>↑</kbd>
  ArrowUp,
  /// On the Mac, this is used for the numpad <kbd>Clear</kbd> key.
  NumLock,
  /// <kbd>0 Ins</kbd> on a keyboard. <kbd>0</kbd> on a phone or remote control
  Numpad0,
  /// <kbd>1 End</kbd> on a keyboard. <kbd>1</kbd> or <kbd>1 QZ</kbd> on a phone or remote control
  Numpad1,
  /// <kbd>2 ↓</kbd> on a keyboard. <kbd>2 ABC</kbd> on a phone or remote control
  Numpad2,
  /// <kbd>3 PgDn</kbd> on a keyboard. <kbd>3 DEF</kbd> on a phone or remote control
  Numpad3,
  /// <kbd>4 ←</kbd> on a keyboard. <kbd>4 GHI</kbd> on a phone or remote control
  Numpad4,
  /// <kbd>5</kbd> on a keyboard. <kbd>5 JKL</kbd> on a phone or remote control
  Numpad5,
  /// <kbd>6 →</kbd> on a keyboard. <kbd>6 MNO</kbd> on a phone or remote control
  Numpad6,
  /// <kbd>7 Home</kbd> on a keyboard. <kbd>7 PQRS</kbd> or <kbd>7 PRS</kbd> on a phone
  /// or remote control
  Numpad7,
  /// <kbd>8 ↑</kbd> on a keyboard. <kbd>8 TUV</kbd> on a phone or remote control
  Numpad8,
  /// <kbd>9 PgUp</kbd> on a keyboard. <kbd>9 WXYZ</kbd> or <kbd>9 WXY</kbd> on a phone
  /// or remote control
  Numpad9,
  /// <kbd>+</kbd>
  NumpadAdd,
  /// Found on the Microsoft Natural Keyboard.
  NumpadBackspace,
  /// <kbd>C</kbd> or <kbd>A</kbd> (All Clear). Also for use with numpads that have a
  /// <kbd>Clear</kbd> key that is separate from the <kbd>NumLock</kbd> key. On the Mac, the
  /// numpad <kbd>Clear</kbd> key is encoded as [`NumLock`].
  ///
  /// [`NumLock`]: Self::NumLock
  NumpadClear,
  /// <kbd>C</kbd> (Clear Entry)
  NumpadClearEntry,
  /// <kbd>,</kbd> (thousands separator). For locales where the thousands separator
  /// is a "." (e.g., Brazil), this key may generate a <kbd>.</kbd>.
  NumpadComma,
  /// <kbd>. Del</kbd>. For locales where the decimal separator is "," (e.g.,
  /// Brazil), this key may generate a <kbd>,</kbd>.
  NumpadDecimal,
  /// <kbd>/</kbd>
  NumpadDivide,
  NumpadEnter,
  /// <kbd>=</kbd>
  NumpadEqual,
  /// <kbd>#</kbd> on a phone or remote control device. This key is typically found
  /// below the <kbd>9</kbd> key and to the right of the <kbd>0</kbd> key.
  NumpadHash,
  /// <kbd>M</kbd> Add current entry to the value stored in memory.
  NumpadMemoryAdd,
  /// <kbd>M</kbd> Clear the value stored in memory.
  NumpadMemoryClear,
  /// <kbd>M</kbd> Replace the current entry with the value stored in memory.
  NumpadMemoryRecall,
  /// <kbd>M</kbd> Replace the value stored in memory with the current entry.
  NumpadMemoryStore,
  /// <kbd>M</kbd> Subtract current entry from the value stored in memory.
  NumpadMemorySubtract,
  /// <kbd>*</kbd> on a keyboard. For use with numpads that provide mathematical
  /// operations (<kbd>+</kbd>, <kbd>-</kbd> <kbd>*</kbd> and <kbd>/</kbd>).
  ///
  /// Use `NumpadStar` for the <kbd>*</kbd> key on phones and remote controls.
  NumpadMultiply,
  /// <kbd>(</kbd> Found on the Microsoft Natural Keyboard.
  NumpadParenLeft,
  /// <kbd>)</kbd> Found on the Microsoft Natural Keyboard.
  NumpadParenRight,
  /// <kbd>*</kbd> on a phone or remote control device.
  ///
  /// This key is typically found below the <kbd>7</kbd> key and to the left of
  /// the <kbd>0</kbd> key.
  ///
  /// Use <kbd>"NumpadMultiply"</kbd> for the <kbd>*</kbd> key on
  /// numeric keypads.
  NumpadStar,
  /// <kbd>-</kbd>
  NumpadSubtract,
  /// <kbd>Esc</kbd> or <kbd>⎋</kbd>
  Escape,
  /// <kbd>Fn</kbd> This is typically a hardware key that does not generate a separate code.
  Fn,
  /// <kbd>FLock</kbd> or <kbd>FnLock</kbd>. Function Lock key. Found on the Microsoft
  /// Natural Keyboard.
  FnLock,
  /// <kbd>PrtScr SysRq</kbd> or <kbd>Print Screen</kbd>
  PrintScreen,
  /// <kbd>Scroll Lock</kbd>
  ScrollLock,
  /// <kbd>Pause Break</kbd>
  Pause,
  /// Some laptops place this key to the left of the <kbd>↑</kbd> key.
  ///
  /// This also the "back" button (triangle) on Android.
  BrowserBack,
  BrowserFavorites,
  /// Some laptops place this key to the right of the <kbd>↑</kbd> key.
  BrowserForward,
  /// The "home" button on Android.
  BrowserHome,
  BrowserRefresh,
  BrowserSearch,
  BrowserStop,
  /// <kbd>Eject</kbd> or <kbd>⏏</kbd>. This key is placed in the function section on some Apple
  /// keyboards.
  Eject,
  /// Sometimes labelled <kbd>My Computer</kbd> on the keyboard
  LaunchApp1,
  /// Sometimes labelled <kbd>Calculator</kbd> on the keyboard
  LaunchApp2,
  LaunchMail,
  MediaPlayPause,
  MediaSelect,
  MediaStop,
  MediaTrackNext,
  MediaTrackPrevious,
  /// This key is placed in the function section on some Apple keyboards, replacing the
  /// <kbd>Eject</kbd> key.
  Power,
  Sleep,
  AudioVolumeDown,
  AudioVolumeMute,
  AudioVolumeUp,
  WakeUp,
  Hyper,
  Turbo,
  Abort,
  Resume,
  Suspend,
  /// Found on Sun’s USB keyboard.
  Again,
  /// Found on Sun’s USB keyboard.
  Copy,
  /// Found on Sun’s USB keyboard.
  Cut,
  /// Found on Sun’s USB keyboard.
  Find,
  /// Found on Sun’s USB keyboard.
  Open,
  /// Found on Sun’s USB keyboard.
  Paste,
  /// Found on Sun’s USB keyboard.
  Props,
  /// Found on Sun’s USB keyboard.
  Select,
  /// Found on Sun’s USB keyboard.
  Undo,
  /// Use for dedicated <kbd>ひらがな</kbd> key found on some Japanese word processing keyboards.
  Hiragana,
  /// Use for dedicated <kbd>カタカナ</kbd> key found on some Japanese word processing keyboards.
  Katakana,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F1,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F2,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F3,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F4,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F5,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F6,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F7,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F8,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F9,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F10,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F11,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F12,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F13,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F14,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F15,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F16,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F17,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F18,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F19,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F20,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F21,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F22,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F23,
  /// General-purpose function key.
  /// Usually found at the top of the keyboard.
  F24,
  /// General-purpose function key.
  F25,
  /// General-purpose function key.
  F26,
  /// General-purpose function key.
  F27,
  /// General-purpose function key.
  F28,
  /// General-purpose function key.
  F29,
  /// General-purpose function key.
  F30,
  /// General-purpose function key.
  F31,
  /// General-purpose function key.
  F32,
  /// General-purpose function key.
  F33,
  /// General-purpose function key.
  F34,
  /// General-purpose function key.
  F35,
}

impl KeyCode {
  /// Return platform specific scancode.
  pub fn to_scancode(self) -> Option<u32> {
    platform_keycode_to_scancode(self)
  }
  /// Return `KeyCode` from platform scancode.
  pub fn from_scancode(scancode: u32) -> KeyCode {
    platform_keycode_from_scancode(scancode)
  }
}

impl FromStr for KeyCode {
  type Err = OsError;
  fn from_str(accelerator_string: &str) -> Result<Self, Self::Err> {
    let keycode = match accelerator_string.to_uppercase().as_str() {
      "`" | "BACKQUOTE" => KeyCode::Backquote,
      "BACKSLASH" => KeyCode::Backslash,
      "[" | "BRACKETLEFT" => KeyCode::BracketLeft,
      "]" | "BRACKETRIGHT" => KeyCode::BracketRight,
      "," | "COMMA" => KeyCode::Comma,
      "0" => KeyCode::Digit0,
      "1" => KeyCode::Digit1,
      "2" => KeyCode::Digit2,
      "3" => KeyCode::Digit3,
      "4" => KeyCode::Digit4,
      "5" => KeyCode::Digit5,
      "6" => KeyCode::Digit6,
      "7" => KeyCode::Digit7,
      "8" => KeyCode::Digit8,
      "9" => KeyCode::Digit9,
      "NUM0" | "NUMPAD0" => KeyCode::Numpad0,
      "NUM1" | "NUMPAD1" => KeyCode::Numpad1,
      "NUM2" | "NUMPAD2" => KeyCode::Numpad2,
      "NUM3" | "NUMPAD3" => KeyCode::Numpad3,
      "NUM4" | "NUMPAD4" => KeyCode::Numpad4,
      "NUM5" | "NUMPAD5" => KeyCode::Numpad5,
      "NUM6" | "NUMPAD6" => KeyCode::Numpad6,
      "NUM7" | "NUMPAD7" => KeyCode::Numpad7,
      "NUM8" | "NUMPAD8" => KeyCode::Numpad8,
      "NUM9" | "NUMPAD9" => KeyCode::Numpad9,
      "=" => KeyCode::Equal,
      "-" => KeyCode::Minus,
      "." | "PERIOD" => KeyCode::Period,
      "'" | "QUOTE" => KeyCode::Quote,
      "\\" => KeyCode::IntlBackslash,
      "A" => KeyCode::KeyA,
      "B" => KeyCode::KeyB,
      "C" => KeyCode::KeyC,
      "D" => KeyCode::KeyD,
      "E" => KeyCode::KeyE,
      "F" => KeyCode::KeyF,
      "G" => KeyCode::KeyG,
      "H" => KeyCode::KeyH,
      "I" => KeyCode::KeyI,
      "J" => KeyCode::KeyJ,
      "K" => KeyCode::KeyK,
      "L" => KeyCode::KeyL,
      "M" => KeyCode::KeyM,
      "N" => KeyCode::KeyN,
      "O" => KeyCode::KeyO,
      "P" => KeyCode::KeyP,
      "Q" => KeyCode::KeyQ,
      "R" => KeyCode::KeyR,
      "S" => KeyCode::KeyS,
      "T" => KeyCode::KeyT,
      "U" => KeyCode::KeyU,
      "V" => KeyCode::KeyV,
      "W" => KeyCode::KeyW,
      "X" => KeyCode::KeyX,
      "Y" => KeyCode::KeyY,
      "Z" => KeyCode::KeyZ,

      ";" | "SEMICOLON" => KeyCode::Semicolon,
      "/" | "SLASH" => KeyCode::Slash,
      "BACKSPACE" => KeyCode::Backspace,
      "CAPSLOCK" => KeyCode::CapsLock,
      "CONTEXTMENU" => KeyCode::ContextMenu,
      "ENTER" => KeyCode::Enter,
      "SPACE" => KeyCode::Space,
      "TAB" => KeyCode::Tab,
      "CONVERT" => KeyCode::Convert,

      "DELETE" => KeyCode::Delete,
      "END" => KeyCode::End,
      "HELP" => KeyCode::Help,
      "HOME" => KeyCode::Home,
      "PAGEDOWN" => KeyCode::PageDown,
      "PAGEUP" => KeyCode::PageUp,

      "DOWN" => KeyCode::ArrowDown,
      "UP" => KeyCode::ArrowUp,
      "LEFT" => KeyCode::ArrowLeft,
      "RIGHT" => KeyCode::ArrowRight,

      "NUMLOCK" => KeyCode::NumLock,
      "NUMADD" | "NUMPADADD" => KeyCode::NumpadAdd,
      "NUMBACKSPACE" | "NUMPADBACKSPACE" => KeyCode::NumpadBackspace,
      "NUMCLEAR" | "NUMPADCLEAR" => KeyCode::NumpadClear,
      "NUMCOMMA" | "NUMPADCOMMA" => KeyCode::NumpadComma,
      "NUMDIVIDE" | "NUMPADDIVIDE" => KeyCode::NumpadDivide,
      "NUMSUBSTRACT" | "NUMPADSUBSTRACT" => KeyCode::NumpadSubtract,
      "NUMENTER" | "NUMPADENTER" => KeyCode::NumpadEnter,

      "ESC" | "ESCAPE" => KeyCode::Escape,
      "FN" => KeyCode::Fn,
      "FNLOCK" => KeyCode::FnLock,
      "PRINTSCREEN" => KeyCode::PrintScreen,
      "SCROLLLOCK" => KeyCode::ScrollLock,

      "PAUSE" => KeyCode::Pause,

      "VOLUMEMUTE" => KeyCode::AudioVolumeMute,
      "VOLUMEDOWN" => KeyCode::AudioVolumeDown,
      "VOLUMEUP" => KeyCode::AudioVolumeUp,
      "MEDIANEXTTRACK" => KeyCode::MediaTrackNext,
      "MEDIAPREVIOUSTRACK" => KeyCode::MediaTrackPrevious,
      "MEDIAPLAYPAUSE" => KeyCode::MediaPlayPause,
      "LAUNCHMAIL" => KeyCode::LaunchMail,

      "SUSPEND" => KeyCode::Suspend,
      "F1" => KeyCode::F1,
      "F2" => KeyCode::F2,
      "F3" => KeyCode::F3,
      "F4" => KeyCode::F4,
      "F5" => KeyCode::F5,
      "F6" => KeyCode::F6,
      "F7" => KeyCode::F7,
      "F8" => KeyCode::F8,
      "F9" => KeyCode::F9,
      "F10" => KeyCode::F10,
      "F11" => KeyCode::F11,
      "F12" => KeyCode::F12,
      "F13" => KeyCode::F13,
      "F14" => KeyCode::F14,
      "F15" => KeyCode::F15,
      "F16" => KeyCode::F16,
      "F17" => KeyCode::F17,
      "F18" => KeyCode::F18,
      "F19" => KeyCode::F19,
      "F20" => KeyCode::F20,
      "F21" => KeyCode::F21,
      "F22" => KeyCode::F22,
      "F23" => KeyCode::F23,
      "F24" => KeyCode::F24,
      "F25" => KeyCode::F25,
      "F26" => KeyCode::F26,
      "F27" => KeyCode::F27,
      "F28" => KeyCode::F28,
      "F29" => KeyCode::F29,
      "F30" => KeyCode::F30,
      "F31" => KeyCode::F31,
      "F32" => KeyCode::F32,
      "F33" => KeyCode::F33,
      "F34" => KeyCode::F34,
      "F35" => KeyCode::F35,
      _ => KeyCode::Unidentified(NativeKeyCode::Unidentified),
    };

    Ok(keycode)
  }
}

impl fmt::Display for KeyCode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      &KeyCode::Unidentified(_) => write!(f, "{:?}", "Unidentified"),
      val => write!(f, "{:?}", val),
    }
  }
}

/// Represents the location of a physical key.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum KeyLocation {
  Standard,
  Left,
  Right,
  Numpad,
}

/// Symbolic name for a keyboard key.
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VirtualKeyCode {
  /// The '1' key over the letters.
  Key1,
  /// The '2' key over the letters.
  Key2,
  /// The '3' key over the letters.
  Key3,
  /// The '4' key over the letters.
  Key4,
  /// The '5' key over the letters.
  Key5,
  /// The '6' key over the letters.
  Key6,
  /// The '7' key over the letters.
  Key7,
  /// The '8' key over the letters.
  Key8,
  /// The '9' key over the letters.
  Key9,
  /// The '0' key over the 'O' and 'P' keys.
  Key0,

  A,
  B,
  C,
  D,
  E,
  F,
  G,
  H,
  I,
  J,
  K,
  L,
  M,
  N,
  O,
  P,
  Q,
  R,
  S,
  T,
  U,
  V,
  W,
  X,
  Y,
  Z,

  /// The Escape key, next to F1.
  Escape,

  F1,
  F2,
  F3,
  F4,
  F5,
  F6,
  F7,
  F8,
  F9,
  F10,
  F11,
  F12,
  F13,
  F14,
  F15,
  F16,
  F17,
  F18,
  F19,
  F20,
  F21,
  F22,
  F23,
  F24,

  /// Print Screen/SysRq.
  Snapshot,
  /// Scroll Lock.
  Scroll,
  /// Pause/Break key, next to Scroll lock.
  Pause,

  /// `Insert`, next to Backspace.
  Insert,
  Home,
  Delete,
  End,
  PageDown,
  PageUp,

  Left,
  Up,
  Right,
  Down,

  /// The Backspace key, right over Enter.
  // TODO: rename
  Back,
  /// The Enter key.
  Return,
  /// The space bar.
  Space,

  /// The "Compose" key on Linux.
  Compose,

  Caret,

  Numlock,
  Numpad0,
  Numpad1,
  Numpad2,
  Numpad3,
  Numpad4,
  Numpad5,
  Numpad6,
  Numpad7,
  Numpad8,
  Numpad9,
  NumpadAdd,
  NumpadDivide,
  NumpadDecimal,
  NumpadComma,
  NumpadEnter,
  NumpadEquals,
  NumpadMultiply,
  NumpadSubtract,

  AbntC1,
  AbntC2,
  Apostrophe,
  Apps,
  Asterisk,
  At,
  Ax,
  Backslash,
  Calculator,
  Capital,
  Colon,
  Comma,
  Convert,
  Equals,
  Grave,
  Kana,
  Kanji,
  LAlt,
  LBracket,
  LControl,
  LShift,
  LWin,
  Mail,
  MediaSelect,
  MediaStop,
  Minus,
  Mute,
  MyComputer,
  // also called "Next"
  NavigateForward,
  // also called "Prior"
  NavigateBackward,
  NextTrack,
  NoConvert,
  OEM102,
  Period,
  PlayPause,
  Plus,
  Power,
  PrevTrack,
  RAlt,
  RBracket,
  RControl,
  RShift,
  RWin,
  Semicolon,
  Slash,
  Sleep,
  Stop,
  Sysrq,
  Tab,
  Underline,
  Unlabeled,
  VolumeDown,
  VolumeUp,
  Wake,
  WebBack,
  WebFavorites,
  WebForward,
  WebHome,
  WebRefresh,
  WebSearch,
  WebStop,
  Yen,
  Copy,
  Paste,
  Cut,
}
