use std::{
  collections::{hash_map::Entry, HashMap},
  fmt,
  hash::Hash,
  str::FromStr,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
  },
};

use crate::platform_impl::{keys, modifiers};
use serde::{Deserialize, Serialize};
#[derive(Debug)]
pub enum Error {
  InvalidHotkey(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Deserialize, Clone, Serialize, PartialEq, Hash, Eq)]
pub struct Hotkey {
  pub modifiers: Vec<Modifier>,
  pub keys: Vec<Key>,
}

impl Hotkey {
  pub fn new(hotkey_string: &str) -> Result<Self> {
    let mut modifiers = Vec::new();
    let mut keys = Vec::new();
    let mut shifted = false;
    for raw in hotkey_string.to_uppercase().split('+') {
      let mut token = raw.trim().to_string();
      if token.is_empty() {
        continue;
      }

      match token.as_str() {
        // command aliases
        "COMMAND" | "CMD" => {
          modifiers.push(Modifier::SUPER);
          continue;
        }
        "CONTROL" => {
          modifiers.push(Modifier::CTRL);
          continue;
        }
        #[cfg(target_os = "macos")]
        "OPTION" => {
          modifiers.push(Modifier::ALT);
          continue;
        }
        "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL" | "CMDORCONTROL" => {
          #[cfg(target_os = "macos")]
          modifiers.push(Modifier::SUPER);
          #[cfg(not(target_os = "macos"))]
          modifiers.push(Modifier::CTRL);
          continue;
        }
        _ => {
          if let Ok(res) = Modifier::from_str(&token) {
            modifiers.push(res);
            continue;
          }
        }
      }

      let mut key = None;

      if token.parse::<usize>().is_ok() {
        token = format!("KEY_{}", token);
      }

      // shift conversions
      match token.as_str() {
        ")" => {
          shifted = true;
          key = Some(Key::KEY_0);
        }
        "!" => {
          shifted = true;
          key = Some(Key::KEY_1);
        }
        "@" => {
          shifted = true;
          key = Some(Key::KEY_2);
        }
        "#" => {
          shifted = true;
          key = Some(Key::KEY_3);
        }
        "$" => {
          shifted = true;
          key = Some(Key::KEY_4);
        }
        "%" => {
          shifted = true;
          key = Some(Key::KEY_5);
        }
        "^" => {
          shifted = true;
          key = Some(Key::KEY_6);
        }
        "&" => {
          shifted = true;
          key = Some(Key::KEY_7);
        }
        "*" => {
          shifted = true;
          key = Some(Key::KEY_8);
        }
        "(" => {
          shifted = true;
          key = Some(Key::KEY_9);
        }
        ":" => {
          shifted = true;
          key = Some(Key::SEMICOLON);
        }
        "<" => {
          shifted = true;
          key = Some(Key::COMMA);
        }
        ">" => {
          shifted = true;
          key = Some(Key::PERIOD);
        }
        "_" => {
          shifted = true;
          key = Some(Key::MINUS);
        }
        "?" => {
          shifted = true;
          key = Some(Key::SLASH);
        }
        "~" => {
          shifted = true;
          key = Some(Key::OPENQUOTE);
        }
        "{" => {
          shifted = true;
          key = Some(Key::OPENBRACKET)
        }
        "|" => {
          shifted = true;
          key = Some(Key::BACKSLASH);
        }
        "}" => {
          shifted = true;
          key = Some(Key::CLOSEBRACKET);
        }
        "+" | "PLUS" => {
          shifted = true;
          key = Some(Key::EQUAL);
        }
        "\"" => {
          shifted = true;
          key = Some(Key::SINGLEQUOTE);
        }
        _ => {}
      }

      // aliases
      if key.is_none() {
        key = match token.as_str() {
          "RETURN" => Some(Key::ENTER),
          "=" => Some(Key::EQUAL),
          "-" => Some(Key::MINUS),
          "'" => Some(Key::SINGLEQUOTE),
          "," => Some(Key::COMMA),
          "." => Some(Key::PERIOD),
          ";" => Some(Key::SEMICOLON),
          "/" => Some(Key::SLASH),
          "`" => Some(Key::OPENQUOTE),
          "[" => Some(Key::OPENBRACKET),
          "\\" => Some(Key::BACKSLASH),
          "]" => Some(Key::CLOSEBRACKET),
          _ => None,
        };
      }

      match key {
        Some(key) => {
          if keys.contains(&key) {
            return Err(Error::InvalidHotkey(format!("duplicated key {}", raw)));
          }
          keys.push(key);
        }
        None => {
          if let Ok(key) = Key::from_str(&token) {
            if keys.contains(&key) {
              return Err(Error::InvalidHotkey(format!("duplicated key {}", raw)));
            }
            keys.push(key);
          } else {
            return Err(Error::InvalidHotkey(format!("unknown key {}", token)));
          }
        }
      }
    }

    if shifted && !modifiers.contains(&Modifier::SHIFT) {
      modifiers.push(Modifier::SHIFT);
    }

    match keys.len() {
      0 => Err(Error::InvalidHotkey(
        "hotkey has no key specified".to_string(),
      )),
      _ => Ok(Hotkey { modifiers, keys }),
    }
  }

  pub fn modifiers_as_flag(&self) -> u32 {
    self.modifiers.iter().fold(0, |acc, x| acc | (*x as u32)) as u32
  }

  pub fn keys_as_flag(&self) -> u32 {
    self.keys.iter().fold(0, |acc, x| acc | (*x as u32)) as u32
  }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(
  Debug, Deserialize, Copy, Clone, Serialize, strum_macros::EnumString, PartialEq, Hash, Eq,
)]
#[repr(u32)]
pub enum Modifier {
  ALT = modifiers::ALT,
  ALTGR = modifiers::ALT_GR,
  CTRL = modifiers::CONTROL,
  SHIFT = modifiers::SHIFT,
  SUPER = modifiers::SUPER,
}

impl fmt::Display for Modifier {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl fmt::Display for Hotkey {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let modifier_string: String = self.modifiers.iter().fold(String::new(), |all, one| {
      if !all.is_empty() {
        format!("{}+{}", all, one)
      } else {
        one.to_string()
      }
    });
    let hotkey_string = {
      if !modifier_string.is_empty() {
        format!(
          "{}+{}",
          modifier_string,
          self
            .keys
            .iter()
            .map(|k| k.to_string())
            .collect::<Vec<String>>()
            .join("\"")
        )
      } else {
        self
          .keys
          .iter()
          .map(|k| k.to_string())
          .collect::<Vec<String>>()
          .join("\"")
      }
    };
    write!(f, "{}", hotkey_string)
  }
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(
  Debug, Deserialize, Copy, Clone, Serialize, strum_macros::EnumString, PartialEq, Hash, Eq,
)]
#[repr(u32)]
pub enum Key {
  BACKSPACE = keys::BACKSPACE,
  TAB = keys::TAB,
  ENTER = keys::ENTER,
  CAPSLOCK = keys::CAPS_LOCK,
  ESCAPE = keys::ESCAPE,
  SPACE = keys::SPACEBAR,
  PAGEUP = keys::PAGE_UP,
  PAGEDOWN = keys::PAGE_DOWN,
  END = keys::END,
  HOME = keys::HOME,
  LEFT = keys::ARROW_LEFT,
  RIGHT = keys::ARROW_RIGHT,
  UP = keys::ARROW_UP,
  DOWN = keys::ARROW_DOWN,
  PRINTSCREEN = keys::PRINT_SCREEN,
  #[cfg(not(target_os = "macos"))]
  INSERT = keys::INSERT,
  CLEAR = keys::CLEAR,
  DELETE = keys::DELETE,
  SCROLLLOCK = keys::SCROLL_LOCK,
  HELP = keys::HELP,
  #[cfg(not(target_os = "macos"))]
  NUMLOCK = keys::NUMLOCK,
  // Media
  VOLUMEMUTE = keys::VOLUME_MUTE,
  VOLUMEDOWN = keys::VOLUME_DOWN,
  VOLUMEUP = keys::VOLUME_UP,
  #[cfg(not(target_os = "macos"))]
  MEDIANEXTTRACK = keys::MEDIA_NEXT,
  #[cfg(not(target_os = "macos"))]
  MEDIAPREVIOUSTRACK = keys::MEDIA_PREV,
  #[cfg(not(target_os = "macos"))]
  MEDIASTOP = keys::MEDIA_STOP,
  #[cfg(not(target_os = "macos"))]
  MEDIAPLAYPAUSE = keys::MEDIA_PLAY_PAUSE,
  #[cfg(not(target_os = "macos"))]
  LAUNCHMAIL = keys::LAUNCH_MAIL,
  // F1-F12
  F1 = keys::F1,
  F2 = keys::F2,
  F3 = keys::F3,
  F4 = keys::F4,
  F5 = keys::F5,
  F6 = keys::F6,
  F7 = keys::F7,
  F8 = keys::F8,
  F9 = keys::F9,
  F10 = keys::F10,
  F11 = keys::F11,
  F12 = keys::F12,
  // Numpad
  NUMADD = keys::ADD,
  NUMSUB = keys::SUBTRACT,
  NUMMULT = keys::MULTIPLY,
  NUMDIV = keys::DIVIDE,
  NUMDEC = keys::DECIMAL,
  #[serde(rename = "0")]
  KEY_0 = keys::KEY_0,
  #[serde(rename = "1")]
  KEY_1 = keys::KEY_1,
  #[serde(rename = "2")]
  KEY_2 = keys::KEY_2,
  #[serde(rename = "3")]
  KEY_3 = keys::KEY_3,
  #[serde(rename = "4")]
  KEY_4 = keys::KEY_4,
  #[serde(rename = "5")]
  KEY_5 = keys::KEY_5,
  #[serde(rename = "6")]
  KEY_6 = keys::KEY_6,
  #[serde(rename = "7")]
  KEY_7 = keys::KEY_7,
  #[serde(rename = "8")]
  KEY_8 = keys::KEY_8,
  #[serde(rename = "9")]
  KEY_9 = keys::KEY_9,
  A = keys::A,
  B = keys::B,
  C = keys::C,
  D = keys::D,
  E = keys::E,
  F = keys::F,
  G = keys::G,
  H = keys::H,
  I = keys::I,
  J = keys::J,
  K = keys::K,
  L = keys::L,
  M = keys::M,
  N = keys::N,
  O = keys::O,
  P = keys::P,
  Q = keys::Q,
  R = keys::R,
  S = keys::S,
  T = keys::T,
  U = keys::U,
  V = keys::V,
  W = keys::W,
  X = keys::X,
  Y = keys::Y,
  Z = keys::Z,
  #[serde(rename = "=")]
  EQUAL = keys::EQUAL,
  #[serde(rename = "-")]
  MINUS = keys::MINUS,
  #[serde(rename = "'")]
  SINGLEQUOTE = keys::SINGLE_QUOTE,
  #[serde(rename = ",")]
  COMMA = keys::COMMA,
  #[serde(rename = ".")]
  PERIOD = keys::PERIOD,
  #[serde(rename = ";")]
  SEMICOLON = keys::SEMICOLON,
  #[serde(rename = "/")]
  SLASH = keys::SLASH,
  #[serde(rename = "`")]
  OPENQUOTE = keys::OPEN_QUOTE,
  #[serde(rename = "[")]
  OPENBRACKET = keys::OPEN_BRACKET,
  #[serde(rename = "\\")]
  BACKSLASH = keys::BACK_SLASH,
  #[serde(rename = "]")]
  CLOSEBRACKET = keys::CLOSE_BRACKET,
}

impl fmt::Display for Key {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:?}", self)
  }
}
