use std::{
  os::raw::{c_int, c_void},
  sync::Once,
};

use crate::{
  error::OsError,
  event::Event,
  event_loop::EventLoopWindowTarget,
  hotkey::{GlobalAccelerator as RootGlobalAccelerator, HotKey},
  keyboard::ModifiersState,
  platform::scancode::KeyCodeExtScancode,
};

use super::{app_state::AppState, event::EventWrapper};

type KeyCallback = unsafe extern "C" fn(c_int, *mut c_void);

unsafe extern "C" fn trampoline<F>(result: c_int, user_data: *mut c_void)
where
  F: FnMut(c_int) + 'static,
{
  let user_data = &mut *(user_data as *mut F);
  user_data(result);
}

fn get_trampoline<F>() -> KeyCallback
where
  F: FnMut(c_int) + 'static,
{
  trampoline::<F>
}

#[link(name = "carbon_hotkey_binding.a", kind = "static")]
extern "C" {
  fn install_event_handler(cb: KeyCallback, data: *mut c_void) -> *mut c_void;
  fn uninstall_event_handler(handler_ref: *mut c_void) -> c_int;
  fn register_hotkey(id: i32, modifier: i32, key: i32) -> *mut c_void;
  fn unregister_hotkey(hotkey_ref: *mut c_void) -> c_int;
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CarbonRef(pub(crate) *mut c_void);
impl CarbonRef {
  pub fn new(start: *mut c_void) -> Self {
    CarbonRef(start)
  }
}

unsafe impl Sync for CarbonRef {}
unsafe impl Send for CarbonRef {}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalAccelerator {
  pub(crate) carbon_ref: Option<CarbonRef>,
  pub(crate) hotkey: HotKey,
}

impl GlobalAccelerator {
  pub(crate) fn new(hotkey: HotKey) -> Self {
    Self {
      carbon_ref: None,
      hotkey,
    }
  }

  pub(crate) fn register(&mut self) -> &mut GlobalAccelerator {
    unsafe {
      let mut converted_modifiers: i32 = 0;
      let modifiers: ModifiersState = self.hotkey.mods.into();
      if modifiers.shift_key() {
        converted_modifiers |= 512;
      }
      if modifiers.super_key() {
        converted_modifiers |= 256;
      }
      if modifiers.alt_key() {
        converted_modifiers |= 2048;
      }
      if modifiers.control_key() {
        converted_modifiers |= 4096;
      }

      // get key scan code
      if let Some(keycode) = self.hotkey.key.to_keycode() {
        let keycode = keycode.first();
        if let Some(keycode) = keycode {
          if let Some(scan_code) = keycode.to_scancode() {
            let handler_ref = register_hotkey(
              self.hotkey.clone().id() as i32,
              converted_modifiers as i32,
              scan_code as i32,
            );
            let saved_callback = Box::into_raw(Box::new(global_accelerator_handler));
            make_accelerator_callback(saved_callback);
            self.carbon_ref = Some(CarbonRef::new(handler_ref));
          }
        }
      }
    }

    println!("done {:?}", self);

    self
  }
}

fn make_accelerator_callback<F>(handler: *mut F)
where
  F: FnMut(i32) + 'static + Sync + Send,
{
  static INIT: Once = Once::new();
  INIT.call_once(|| unsafe {
    let cb = get_trampoline::<F>();
    install_event_handler(cb, handler as *mut c_void);
  });
}

fn global_accelerator_handler(item_id: i32) {
  AppState::queue_event(EventWrapper::StaticEvent(Event::GlobalHotKeyEvent(
    item_id as u16,
  )));
}

pub fn register_global_accelerators<T>(
  _window_target: &EventLoopWindowTarget<T>,
  accelerators: &mut Vec<RootGlobalAccelerator>,
) {
  for accel in accelerators {
    accel.0.register();
  }
}

// todo implement drop?
