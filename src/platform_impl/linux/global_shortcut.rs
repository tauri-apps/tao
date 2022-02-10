use crate::{
  accelerator::{Accelerator, AcceleratorId},
  event_loop::EventLoopWindowTarget,
  global_shortcut::{GlobalShortcut as RootGlobalShortcut, ShortcutManagerError},
};

#[derive(Debug)]
pub struct ShortcutManager {
  tx: glib::Sender<GlobalShortcutEvent>,
}

impl ShortcutManager {
  pub(crate) fn new<T>(window_target: &EventLoopWindowTarget<T>) -> Self {
    Self {
      tx: window_target.p.shortcut_requests_tx.clone(),
    }
  }

  pub(crate) fn register(
    &mut self,
    accelerator: Accelerator,
  ) -> Result<RootGlobalShortcut, ShortcutManagerError> {
    if let Some(key) = get_gtk_key(&accelerator) {
      let id = accelerator.clone().id();
      if let Err(e) = self.tx.send(GlobalShortcutEvent::Register((id, key))) {
        log::warn!(
          "Failed to send global shortcut event to event loop channel: {}",
          e
        );
      }
      Ok(RootGlobalShortcut(GlobalShortcut { accelerator }))
    } else {
      Err(ShortcutManagerError::InvalidAccelerator(
        "Failed to convert KeyCode to gdk::Key".into(),
      ))
    }
  }

  pub(crate) fn unregister_all(&mut self) -> Result<(), ShortcutManagerError> {
    if let Err(e) = self.tx.send(GlobalShortcutEvent::UnRegisterAll) {
      log::warn!(
        "Failed to send global shortcut event to event loop channel: {}",
        e
      );
    }
    Ok(())
  }

  pub(crate) fn unregister(
    &self,
    shortcut: RootGlobalShortcut,
  ) -> Result<(), ShortcutManagerError> {
    if let Err(e) = self
      .tx
      .send(GlobalShortcutEvent::UnRegister(shortcut.0.accelerator.id()))
    {
      log::warn!(
        "Failed to send global shortcut event to event loop channel: {}",
        e
      );
    }
    Ok(())
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalShortcut {
  pub(crate) accelerator: Accelerator,
}

impl GlobalShortcut {
  pub fn id(&self) -> AcceleratorId {
    self.accelerator.clone().id()
  }
}

pub enum GlobalShortcutEvent {
  Register((AcceleratorId, String)),
  UnRegister(AcceleratorId),
  UnRegisterAll,
}

fn get_gtk_key(key: &Accelerator) -> Option<String> {
  let mut result = String::new();

  let mods = key.mods;
  if mods.shift_key() {
    result += "<Shift>";
  }
  if mods.control_key() {
    result += "<Ctrl>";
  }
  if mods.alt_key() {
    result += "<Alt>";
  }
  if mods.super_key() {
    result += "<Super>";
  }

  if let Some(k) = super::keyboard::key_to_raw_key(&key.key) {
    if let Some(name) = k.name() {
      result += &name;
      Some(result)
    } else {
      None
    }
  } else {
    None
  }
}
