use std::os::raw::{c_int, c_void};

use crate::{
  accelerator::{Accelerator, AcceleratorId},
  event::Event,
  event_loop::EventLoopWindowTarget,
  global_shortcut::{GlobalShortcut as RootGlobalShortcut, ShortcutManagerError},
};

use super::{app_state::AppState, event::EventWrapper};

type KeyCallback = unsafe extern "C" fn(c_int, *mut c_void);
#[derive(Debug, Clone)]
pub struct ShortcutManager {
  shortcuts: Vec<GlobalShortcut>,
  event_handler: *mut c_void,
}

impl ShortcutManager {
  pub(crate) fn new<T>(_window_target: &EventLoopWindowTarget<T>) -> Self {
    let saved_callback = Box::into_raw(Box::new(global_accelerator_handler));
    let event_handler = make_accelerator_callback(saved_callback);
    ShortcutManager {
      event_handler,
      shortcuts: Vec::new(),
    }
  }

  pub(crate) fn unregister_all(&self) -> Result<(), ShortcutManagerError> {
    for shortcut in &self.shortcuts {
      shortcut.unregister();
    }
    Ok(())
  }

  pub(crate) fn unregister(
    &self,
    shortcut: RootGlobalShortcut,
  ) -> Result<(), ShortcutManagerError> {
    shortcut.0.unregister();
    Ok(())
  }

  pub(crate) fn register(
    &mut self,
    accelerator: Accelerator,
  ) -> Result<RootGlobalShortcut, ShortcutManagerError> {
    unsafe {
      let mut converted_modifiers: i32 = 0;
      if accelerator.mods.shift_key() {
        converted_modifiers |= 512;
      }
      if accelerator.mods.super_key() {
        converted_modifiers |= 256;
      }
      if accelerator.mods.alt_key() {
        converted_modifiers |= 2048;
      }
      if accelerator.mods.control_key() {
        converted_modifiers |= 4096;
      }

      // we get only 1 keycode as we don't generate it for the modifier
      // it's safe to use first()
      if let Some(scan_code) = accelerator.key.to_scancode() {
        #[cfg(debug_assertions)]
        println!("register {:?}", accelerator);
        // register hotkey
        let handler_ref = register_hotkey(
          accelerator.clone().id().0 as i32,
          converted_modifiers as i32,
          scan_code as i32,
        );
        let shortcut = GlobalShortcut {
          accelerator,
          carbon_ref: CarbonRef::new(handler_ref),
        };
        self.shortcuts.push(shortcut.clone());
        return Ok(RootGlobalShortcut(shortcut));
      }
    }

    Err(ShortcutManagerError::InvalidAccelerator(
      "Invalid accelerator".into(),
    ))
  }

  // connect_event_loop is not needed on macos
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalShortcut {
  pub(crate) carbon_ref: CarbonRef,
  pub(crate) accelerator: Accelerator,
}

impl GlobalShortcut {
  pub fn id(&self) -> AcceleratorId {
    self.accelerator.clone().id()
  }
}

impl GlobalShortcut {
  pub(crate) fn unregister(&self) {
    unsafe { unregister_hotkey(self.carbon_ref.0) };
  }
}

fn make_accelerator_callback<F>(handler: *mut F) -> *mut c_void
where
  F: FnMut(i32) + 'static + Sync + Send,
{
  let cb = get_trampoline::<F>();
  unsafe { install_event_handler(cb, handler as *mut c_void) }
}

fn global_accelerator_handler(item_id: i32) {
  AppState::queue_event(EventWrapper::StaticEvent(Event::GlobalShortcutEvent(
    AcceleratorId(item_id as u16),
  )));
}

impl Drop for ShortcutManager {
  fn drop(&mut self) {
    self.unregister_all().unwrap();
    unsafe {
      uninstall_event_handler(self.event_handler);
    }
  }
}
