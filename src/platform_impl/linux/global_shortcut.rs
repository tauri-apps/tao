use super::window::{WindowId, WindowRequest};
use crate::{
  accelerator::Accelerator,
  event_loop::EventLoopWindowTarget,
  keyboard::{Key, ModifiersState},
  platform::global_shortcut::{GlobalShortcut as RootGlobalShortcut, ShortcutManagerError},
};
use std::{
  collections::HashMap,
  mem::MaybeUninit,
  ptr,
  sync::{
    mpsc,
    mpsc::{Receiver, Sender},
    Arc, Mutex,
  },
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
  method_sender: mpsc::Sender<HotkeyMessage>,
  method_receiver: mpsc::Receiver<HotkeyMessage>,
}

impl ShortcutManager {
  pub(crate) fn register(
    &mut self,
    accelerator: Accelerator,
  ) -> Result<RootGlobalShortcut, ShortcutManagerError> {
    let keycode = get_x11_scancode_from_hotkey(accelerator.key.clone()) as u32;

    let mut converted_modifiers: u32 = 0;
    let modifiers: ModifiersState = accelerator.mods.into();
    if modifiers.shift_key() {
      converted_modifiers |= xlib::ShiftMask;
    }
    if modifiers.super_key() {
      converted_modifiers |= xlib::Mod4Mask;
    }
    if modifiers.alt_key() {
      converted_modifiers |= xlib::Mod1Mask;
    }
    if modifiers.control_key() {
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

    match self.method_receiver.recv() {
      Ok(HotkeyMessage::RegisterHotkeyResult(Ok(id))) => {
        self
          .shortcuts
          .lock()
          .unwrap()
          .insert(id, accelerator.clone().id() as u32);
        let shortcut = GlobalShortcut { accelerator };
        return Ok(RootGlobalShortcut(shortcut));
      }
      Ok(HotkeyMessage::RegisterHotkeyResult(Err(err))) => Err(err),
      Err(err) => Err(ShortcutManagerError::InvalidAccelerator(err.to_string())),
      _ => Err(ShortcutManagerError::InvalidAccelerator(
        "Unknown error".into(),
      )),
    }
  }

  pub(crate) fn unregister_all(&self) -> Result<(), ShortcutManagerError> {
    Ok(())
  }

  pub(crate) fn new<T>(_window_target: &EventLoopWindowTarget<T>) -> Self {
    println!("here connecting...");
    let window_id = WindowId::dummy();
    let hotkeys = ListenerMap::default();
    let hotkey_map = hotkeys.clone();

    let event_loop_channel = _window_target.p.window_requests_tx.clone();

    let (method_sender, thread_receiver) = mpsc::channel();
    let (thread_sender, method_receiver) = mpsc::channel();

    std::thread::spawn(move || {
      let event_loop_channel = event_loop_channel.clone();
      let xlib = xlib::Xlib::open().unwrap();
      unsafe {
        let display = (xlib.XOpenDisplay)(ptr::null());
        let root = (xlib.XDefaultRootWindow)(display);

        // Only trigger key release at end of repeated keys
        let mut supported_rtrn: i32 = std::mem::MaybeUninit::uninit().assume_init();
        (xlib.XkbSetDetectableAutoRepeat)(display, 1, &mut supported_rtrn);

        (xlib.XSelectInput)(display, root, xlib::KeyReleaseMask);
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
                  eprintln!("hotkey: thread_sender.send error {}", err);
                }
              } else if let Err(err) = thread_sender.send(HotkeyMessage::RegisterHotkeyResult(Ok(
                (keycode, modifiers),
              ))) {
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
                  eprintln!("hotkey: thread_sender.send error {}", err);
                }
              }
            }
            Ok(HotkeyMessage::DropThread) => {
              (xlib.XCloseDisplay)(display);
              return;
            }
            Err(err) => {
              if let std::sync::mpsc::TryRecvError::Disconnected = err {
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalShortcut {
  pub(crate) accelerator: Accelerator,
}
type ListenerId = (i32, u32);
type ListenerMap = Arc<Mutex<HashMap<ListenerId, u32>>>;

impl GlobalShortcut {
  pub(crate) fn unregister(&self) {
    unsafe {}
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

fn get_x11_scancode_from_hotkey(key: Key) -> u32 {
  match key {
    Key::Character(char) => {
      // FIXME: convert string to `char` then u32
      match char.to_uppercase().as_str() {
        "A" => 'A' as u32,
        "B" => 'B' as u32,
        "C" => 'C' as u32,
        "D" => 'D' as u32,
        "E" => 'E' as u32,
        "F" => 'F' as u32,
        "G" => 'G' as u32,
        "H" => 'H' as u32,
        "I" => 'I' as u32,
        "J" => 'J' as u32,
        "K" => 'K' as u32,
        "L" => 'L' as u32,
        "M" => 'M' as u32,
        "N" => 'N' as u32,
        "O" => 'O' as u32,
        "P" => 'P' as u32,
        "Q" => 'Q' as u32,
        "R" => 'R' as u32,
        "S" => 'S' as u32,
        "T" => 'T' as u32,
        "U" => 'U' as u32,
        "V" => 'V' as u32,
        "W" => 'W' as u32,
        "X" => 'X' as u32,
        "Y" => 'Y' as u32,
        "Z" => 'Z' as u32,
        _ => 0,
      }
    }
    Key::Unidentified(_) => 0,
    Key::Dead(_) => todo!(),
    Key::Alt => todo!(),
    Key::AltGraph => todo!(),
    Key::CapsLock => todo!(),
    Key::Control => todo!(),
    Key::Fn => todo!(),
    Key::FnLock => todo!(),
    Key::NumLock => todo!(),
    Key::ScrollLock => todo!(),
    Key::Shift => todo!(),
    Key::Symbol => todo!(),
    Key::SymbolLock => todo!(),
    Key::Hyper => todo!(),
    Key::Super => todo!(),
    Key::Enter => todo!(),
    Key::Tab => todo!(),
    Key::Space => todo!(),
    Key::ArrowDown => todo!(),
    Key::ArrowLeft => todo!(),
    Key::ArrowRight => todo!(),
    Key::ArrowUp => todo!(),
    Key::End => todo!(),
    Key::Home => todo!(),
    Key::PageDown => todo!(),
    Key::PageUp => todo!(),
    Key::Backspace => todo!(),
    Key::Clear => todo!(),
    Key::Copy => todo!(),
    Key::CrSel => todo!(),
    Key::Cut => todo!(),
    Key::Delete => todo!(),
    Key::EraseEof => todo!(),
    Key::ExSel => todo!(),
    Key::Insert => todo!(),
    Key::Paste => todo!(),
    Key::Redo => todo!(),
    Key::Undo => todo!(),
    Key::Accept => todo!(),
    Key::Again => todo!(),
    Key::Attn => todo!(),
    Key::Cancel => todo!(),
    Key::ContextMenu => todo!(),
    Key::Escape => todo!(),
    Key::Execute => todo!(),
    Key::Find => todo!(),
    Key::Help => todo!(),
    Key::Pause => todo!(),
    Key::Play => todo!(),
    Key::Props => todo!(),
    Key::Select => todo!(),
    Key::ZoomIn => todo!(),
    Key::ZoomOut => todo!(),
    Key::BrightnessDown => todo!(),
    Key::BrightnessUp => todo!(),
    Key::Eject => todo!(),
    Key::LogOff => todo!(),
    Key::Power => todo!(),
    Key::PowerOff => todo!(),
    Key::PrintScreen => todo!(),
    Key::Hibernate => todo!(),
    Key::Standby => todo!(),
    Key::WakeUp => todo!(),
    Key::AllCandidates => todo!(),
    Key::Alphanumeric => todo!(),
    Key::CodeInput => todo!(),
    Key::Compose => todo!(),
    Key::Convert => todo!(),
    Key::FinalMode => todo!(),
    Key::GroupFirst => todo!(),
    Key::GroupLast => todo!(),
    Key::GroupNext => todo!(),
    Key::GroupPrevious => todo!(),
    Key::ModeChange => todo!(),
    Key::NextCandidate => todo!(),
    Key::NonConvert => todo!(),
    Key::PreviousCandidate => todo!(),
    Key::Process => todo!(),
    Key::SingleCandidate => todo!(),
    Key::HangulMode => todo!(),
    Key::HanjaMode => todo!(),
    Key::JunjaMode => todo!(),
    Key::Eisu => todo!(),
    Key::Hankaku => todo!(),
    Key::Hiragana => todo!(),
    Key::HiraganaKatakana => todo!(),
    Key::KanaMode => todo!(),
    Key::KanjiMode => todo!(),
    Key::Katakana => todo!(),
    Key::Romaji => todo!(),
    Key::Zenkaku => todo!(),
    Key::ZenkakuHankaku => todo!(),
    Key::Soft1 => todo!(),
    Key::Soft2 => todo!(),
    Key::Soft3 => todo!(),
    Key::Soft4 => todo!(),
    Key::ChannelDown => todo!(),
    Key::ChannelUp => todo!(),
    Key::Close => todo!(),
    Key::MailForward => todo!(),
    Key::MailReply => todo!(),
    Key::MailSend => todo!(),
    Key::MediaClose => todo!(),
    Key::MediaFastForward => todo!(),
    Key::MediaPause => todo!(),
    Key::MediaPlay => todo!(),
    Key::MediaPlayPause => todo!(),
    Key::MediaRecord => todo!(),
    Key::MediaRewind => todo!(),
    Key::MediaStop => todo!(),
    Key::MediaTrackNext => todo!(),
    Key::MediaTrackPrevious => todo!(),
    Key::New => todo!(),
    Key::Open => todo!(),
    Key::Print => todo!(),
    Key::Save => todo!(),
    Key::SpellCheck => todo!(),
    Key::Key11 => todo!(),
    Key::Key12 => todo!(),
    Key::AudioBalanceLeft => todo!(),
    Key::AudioBalanceRight => todo!(),
    Key::AudioBassBoostDown => todo!(),
    Key::AudioBassBoostToggle => todo!(),
    Key::AudioBassBoostUp => todo!(),
    Key::AudioFaderFront => todo!(),
    Key::AudioFaderRear => todo!(),
    Key::AudioSurroundModeNext => todo!(),
    Key::AudioTrebleDown => todo!(),
    Key::AudioTrebleUp => todo!(),
    Key::AudioVolumeDown => todo!(),
    Key::AudioVolumeUp => todo!(),
    Key::AudioVolumeMute => todo!(),
    Key::MicrophoneToggle => todo!(),
    Key::MicrophoneVolumeDown => todo!(),
    Key::MicrophoneVolumeUp => todo!(),
    Key::MicrophoneVolumeMute => todo!(),
    Key::SpeechCorrectionList => todo!(),
    Key::SpeechInputToggle => todo!(),
    Key::LaunchApplication1 => todo!(),
    Key::LaunchApplication2 => todo!(),
    Key::LaunchCalendar => todo!(),
    Key::LaunchContacts => todo!(),
    Key::LaunchMail => todo!(),
    Key::LaunchMediaPlayer => todo!(),
    Key::LaunchMusicPlayer => todo!(),
    Key::LaunchPhone => todo!(),
    Key::LaunchScreenSaver => todo!(),
    Key::LaunchSpreadsheet => todo!(),
    Key::LaunchWebBrowser => todo!(),
    Key::LaunchWebCam => todo!(),
    Key::LaunchWordProcessor => todo!(),
    Key::BrowserBack => todo!(),
    Key::BrowserFavorites => todo!(),
    Key::BrowserForward => todo!(),
    Key::BrowserHome => todo!(),
    Key::BrowserRefresh => todo!(),
    Key::BrowserSearch => todo!(),
    Key::BrowserStop => todo!(),
    Key::AppSwitch => todo!(),
    Key::Call => todo!(),
    Key::Camera => todo!(),
    Key::CameraFocus => todo!(),
    Key::EndCall => todo!(),
    Key::GoBack => todo!(),
    Key::GoHome => todo!(),
    Key::HeadsetHook => todo!(),
    Key::LastNumberRedial => todo!(),
    Key::Notification => todo!(),
    Key::MannerMode => todo!(),
    Key::VoiceDial => todo!(),
    Key::TV => todo!(),
    Key::TV3DMode => todo!(),
    Key::TVAntennaCable => todo!(),
    Key::TVAudioDescription => todo!(),
    Key::TVAudioDescriptionMixDown => todo!(),
    Key::TVAudioDescriptionMixUp => todo!(),
    Key::TVContentsMenu => todo!(),
    Key::TVDataService => todo!(),
    Key::TVInput => todo!(),
    Key::TVInputComponent1 => todo!(),
    Key::TVInputComponent2 => todo!(),
    Key::TVInputComposite1 => todo!(),
    Key::TVInputComposite2 => todo!(),
    Key::TVInputHDMI1 => todo!(),
    Key::TVInputHDMI2 => todo!(),
    Key::TVInputHDMI3 => todo!(),
    Key::TVInputHDMI4 => todo!(),
    Key::TVInputVGA1 => todo!(),
    Key::TVMediaContext => todo!(),
    Key::TVNetwork => todo!(),
    Key::TVNumberEntry => todo!(),
    Key::TVPower => todo!(),
    Key::TVRadioService => todo!(),
    Key::TVSatellite => todo!(),
    Key::TVSatelliteBS => todo!(),
    Key::TVSatelliteCS => todo!(),
    Key::TVSatelliteToggle => todo!(),
    Key::TVTerrestrialAnalog => todo!(),
    Key::TVTerrestrialDigital => todo!(),
    Key::TVTimer => todo!(),
    Key::AVRInput => todo!(),
    Key::AVRPower => todo!(),
    Key::ColorF0Red => todo!(),
    Key::ColorF1Green => todo!(),
    Key::ColorF2Yellow => todo!(),
    Key::ColorF3Blue => todo!(),
    Key::ColorF4Grey => todo!(),
    Key::ColorF5Brown => todo!(),
    Key::ClosedCaptionToggle => todo!(),
    Key::Dimmer => todo!(),
    Key::DisplaySwap => todo!(),
    Key::DVR => todo!(),
    Key::Exit => todo!(),
    Key::FavoriteClear0 => todo!(),
    Key::FavoriteClear1 => todo!(),
    Key::FavoriteClear2 => todo!(),
    Key::FavoriteClear3 => todo!(),
    Key::FavoriteRecall0 => todo!(),
    Key::FavoriteRecall1 => todo!(),
    Key::FavoriteRecall2 => todo!(),
    Key::FavoriteRecall3 => todo!(),
    Key::FavoriteStore0 => todo!(),
    Key::FavoriteStore1 => todo!(),
    Key::FavoriteStore2 => todo!(),
    Key::FavoriteStore3 => todo!(),
    Key::Guide => todo!(),
    Key::GuideNextDay => todo!(),
    Key::GuidePreviousDay => todo!(),
    Key::Info => todo!(),
    Key::InstantReplay => todo!(),
    Key::Link => todo!(),
    Key::ListProgram => todo!(),
    Key::LiveContent => todo!(),
    Key::Lock => todo!(),
    Key::MediaApps => todo!(),
    Key::MediaAudioTrack => todo!(),
    Key::MediaLast => todo!(),
    Key::MediaSkipBackward => todo!(),
    Key::MediaSkipForward => todo!(),
    Key::MediaStepBackward => todo!(),
    Key::MediaStepForward => todo!(),
    Key::MediaTopMenu => todo!(),
    Key::NavigateIn => todo!(),
    Key::NavigateNext => todo!(),
    Key::NavigateOut => todo!(),
    Key::NavigatePrevious => todo!(),
    Key::NextFavoriteChannel => todo!(),
    Key::NextUserProfile => todo!(),
    Key::OnDemand => todo!(),
    Key::Pairing => todo!(),
    Key::PinPDown => todo!(),
    Key::PinPMove => todo!(),
    Key::PinPToggle => todo!(),
    Key::PinPUp => todo!(),
    Key::PlaySpeedDown => todo!(),
    Key::PlaySpeedReset => todo!(),
    Key::PlaySpeedUp => todo!(),
    Key::RandomToggle => todo!(),
    Key::RcLowBattery => todo!(),
    Key::RecordSpeedNext => todo!(),
    Key::RfBypass => todo!(),
    Key::ScanChannelsToggle => todo!(),
    Key::ScreenModeNext => todo!(),
    Key::Settings => todo!(),
    Key::SplitScreenToggle => todo!(),
    Key::STBInput => todo!(),
    Key::STBPower => todo!(),
    Key::Subtitle => todo!(),
    Key::Teletext => todo!(),
    Key::VideoModeNext => todo!(),
    Key::Wink => todo!(),
    Key::ZoomToggle => todo!(),
    Key::F1 => keysym::XK_F1,
    Key::F2 => todo!(),
    Key::F3 => todo!(),
    Key::F4 => todo!(),
    Key::F5 => todo!(),
    Key::F6 => todo!(),
    Key::F7 => todo!(),
    Key::F8 => todo!(),
    Key::F9 => todo!(),
    Key::F10 => todo!(),
    Key::F11 => todo!(),
    Key::F12 => keysym::XK_F12,
    Key::F13 => keysym::XK_F13,
    Key::F14 => todo!(),
    Key::F15 => todo!(),
    Key::F16 => todo!(),
    Key::F17 => todo!(),
    Key::F18 => todo!(),
    Key::F19 => todo!(),
    Key::F20 => todo!(),
    Key::F21 => todo!(),
    Key::F22 => todo!(),
    Key::F23 => todo!(),
    Key::F24 => todo!(),
    Key::F25 => todo!(),
    Key::F26 => todo!(),
    Key::F27 => todo!(),
    Key::F28 => todo!(),
    Key::F29 => todo!(),
    Key::F30 => todo!(),
    Key::F31 => todo!(),
    Key::F32 => todo!(),
    Key::F33 => todo!(),
    Key::F34 => todo!(),
    Key::F35 => todo!(),
  }
}
