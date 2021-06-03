use gdk::{keys::constants::*, ModifierType, EventKey};
use crate::{keyboard::{Key, ModifiersState, KeyLocation, NativeKeyCode}, event::{ElementState, KeyEvent}};

pub type RawKey = gdk::keys::Key;

#[allow(clippy::just_underscores_and_digits, non_upper_case_globals)]
pub fn raw_key_to_key(gdk_key: RawKey) -> Option<Key<'static>> {

    let unicode = gdk_key.to_unicode();

    let key = match gdk_key {
        Escape => Some(Key::Escape),
        BackSpace => Some(Key::Backspace),
        Tab | ISO_Left_Tab => Some(Key::Tab),
        Return => Some(Key::Enter),
        Control_L | Control_R => Some(Key::Control),
        Alt_L | Alt_R => Some(Key::Alt),
        Shift_L | Shift_R => Some(Key::Shift),
        // TODO: investigate mapping. Map Meta_[LR]?
        Super_L | Super_R => Some(Key::Super),
        Caps_Lock => Some(Key::CapsLock),
        F1 => Some(Key::F1),
        F2 => Some(Key::F2),
        F3 => Some(Key::F3),
        F4 => Some(Key::F4),
        F5 => Some(Key::F5),
        F6 => Some(Key::F6),
        F7 => Some(Key::F7),
        F8 => Some(Key::F8),
        F9 => Some(Key::F9),
        F10 => Some(Key::F10),
        F11 => Some(Key::F11),
        F12 => Some(Key::F12),

        Print => Some(Key::PrintScreen),
        Scroll_Lock => Some(Key::ScrollLock),
        // Pause/Break not audio.
        Pause => Some(Key::Pause),

        Insert => Some(Key::Insert),
        Delete => Some(Key::Delete),
        Home => Some(Key::Home),
        End => Some(Key::End),
        Page_Up => Some(Key::PageUp),
        Page_Down => Some(Key::PageDown),
        Num_Lock => Some(Key::NumLock),

        Up => Some(Key::ArrowUp),
        Down => Some(Key::ArrowDown),
        Left => Some(Key::ArrowLeft),
        Right => Some(Key::ArrowRight),
        Clear => Some(Key::Clear),

        Menu => Some(Key::ContextMenu),
        WakeUp => Some(Key::WakeUp),
        Launch0 => Some(Key::LaunchApplication1),
        Launch1 => Some(Key::LaunchApplication2),
        ISO_Level3_Shift => Some(Key::AltGraph),

        KP_Begin => Some(Key::Clear),
        KP_Delete => Some(Key::Delete),
        KP_Down => Some(Key::ArrowDown),
        KP_End => Some(Key::End),
        KP_Enter => Some(Key::Enter),
        KP_F1 => Some(Key::F1),
        KP_F2 => Some(Key::F2),
        KP_F3 => Some(Key::F3),
        KP_F4 => Some(Key::F4),
        KP_Home => Some(Key::Home),
        KP_Insert => Some(Key::Insert),
        KP_Left => Some(Key::ArrowLeft),
        KP_Page_Down => Some(Key::PageDown),
        KP_Page_Up => Some(Key::PageUp),
        KP_Right => Some(Key::ArrowRight),
        // KP_Separator? What does it map to?
        KP_Tab => Some(Key::Tab),
        KP_Up => Some(Key::ArrowUp),
        // TODO: more mappings (media etc)
        _ => return None,
    };

    key
}

pub fn raw_key_to_location(raw: RawKey) -> KeyLocation {
    match raw {
        Control_L | Shift_L | Alt_L | Super_L | Meta_L => KeyLocation::Left,
        Control_R | Shift_R | Alt_R | Super_R | Meta_R => KeyLocation::Right,
        KP_0 | KP_1 | KP_2 | KP_3 | KP_4 | KP_5 | KP_6 | KP_7 | KP_8 | KP_9 | KP_Add | KP_Begin
        | KP_Decimal | KP_Delete | KP_Divide | KP_Down | KP_End | KP_Enter | KP_Equal | KP_F1
        | KP_F2 | KP_F3 | KP_F4 | KP_Home | KP_Insert | KP_Left | KP_Multiply | KP_Page_Down
        | KP_Page_Up | KP_Right | KP_Separator | KP_Space | KP_Subtract | KP_Tab | KP_Up => {
            KeyLocation::Numpad
        }
        _ => KeyLocation::Standard,
    }
}

const MODIFIER_MAP: &[(ModifierType, ModifiersState)] = &[
    (ModifierType::SHIFT_MASK, ModifiersState::SHIFT),
    (ModifierType::MOD1_MASK, ModifiersState::ALT),
    (ModifierType::CONTROL_MASK, ModifiersState::CONTROL),
    (ModifierType::SUPER_MASK, ModifiersState::SUPER),
];

pub(crate) fn get_modifiers(modifiers: ModifierType) -> ModifiersState {
    let mut result = ModifiersState::empty();
    for &(gdk_mod, modifier) in MODIFIER_MAP {
        if modifiers.contains(gdk_mod) {
            result |= modifier;
        }
    }
    result
}

fn make_key_event(key: &EventKey, repeat: bool, state: ElementState) -> KeyEvent {
    let keyval = key.get_keyval();
    let hardware_keycode = key.get_hardware_keycode();

    //let keycode = hardware_keycode_to_keyval(hardware_keycode).unwrap_or_else(|| keyval.clone());

    let gdk_text = gdk::keys::keyval_to_unicode(*keyval);
    let mods = get_modifiers(key.get_state());
    let key = raw_key_to_key(keyval).unwrap_or_else(|| {
        if let Some(char(c)) = gdk_text {
            if c >= ' ' && c != '\x7f' {
                Key::Character(&c.to_string())
            } else {
                Key::Unidentified(NativeKeyCode::Gtk(hardware_keycode))
            }
        } else {
            Key::Unidentified(NativeKeyCode::Gtk(hardware_keycode))
        }
    });
    //let code = hardware_keycode_to_code(hardware_keycode);
    //let location = raw_key_to_location(keycode);
    let is_composing = false;

}
