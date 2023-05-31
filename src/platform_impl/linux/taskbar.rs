use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

mod xapps;

pub struct TaskbarIndicator {
  xapps: Option<xapps::Manager>,
  progress: f64,
  progress_visible: bool,
  needs_attention: bool,
}

#[allow(dead_code)]
impl TaskbarIndicator {
  pub fn new(
    window: RawWindowHandle,
    d_handle: RawDisplayHandle,
  ) -> Result<Self, Box<dyn std::error::Error>> {
    Ok(Self {
      xapps: xapps::Manager::new(window, d_handle),
      progress: 0.0,
      progress_visible: false,
      needs_attention: false,
    })
  }

  pub fn set_progress(&mut self, progress: f64) -> Result<(), Box<dyn std::error::Error>> {
    self.progress = progress;

    self.xapps.as_mut().unwrap().set_progress(progress)?;
    Ok(())
  }

  pub fn set_progress_state(&mut self, state: bool) -> Result<(), Box<dyn std::error::Error>> {
    self.progress_visible = state;

    self.xapps.as_mut().unwrap().set_progress_visible(state)?;
    Ok(())
  }

  pub fn needs_attention(
    &mut self,
    needs_attention: bool,
  ) -> Result<(), Box<dyn std::error::Error>> {
    self.needs_attention = needs_attention;

    self.xapps.as_mut().unwrap().needs_attention(needs_attention)?;
    Ok(())
  }
}
