use crate::{
  accelerator::{Accelerator, AcceleratorId},
  event_loop::EventLoopWindowTarget,
  global_shortcut::{GlobalShortcut as RootGlobalShortcut, ShortcutManagerError},
};

#[derive(Debug)]
pub struct ShortcutManager {
  tx: glib::Sender<ShortcutEvent>,
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
    if let Err(e) = self.tx.send(ShortcutEvent::Register(accelerator.clone())) {
      log::warn!(
        "Failed to send global shortcut event to event loop channel: {}",
        e
      );
    }
    Ok(RootGlobalShortcut(GlobalShortcut { accelerator }))
  }

  pub(crate) fn unregister_all(&mut self) -> Result<(), ShortcutManagerError> {
    if let Err(e) = self.tx.send(ShortcutEvent::UnRegisterAll) {
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
      .send(ShortcutEvent::UnRegister(shortcut.0.accelerator))
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

pub enum ShortcutEvent {
  Register(Accelerator),
  UnRegister(Accelerator),
  UnRegisterAll,
}
