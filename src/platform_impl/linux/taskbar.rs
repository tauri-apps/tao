mod unity;

pub struct TaskbarIndicator {
  unity: Option<unity::Manager>,
  progress: f64,
  progress_visible: bool,
  needs_attention: bool,
}

#[allow(dead_code)]
impl TaskbarIndicator {
  pub fn new() -> Self {
    Self {
      unity: None,
      progress: 0.0,
      progress_visible: false,
      needs_attention: false,
    }
  }

  pub fn set_unity_app_uri(&mut self, uri: impl AsRef<str>) -> Result<(), zbus::Error> {
    let mut unity = unity::Manager::new(uri.as_ref().to_owned())?;

    unity.set_progress(self.progress).unwrap_or(());
    unity
      .set_progress_visible(self.progress_visible)
      .unwrap_or(());
    unity.needs_attention(self.needs_attention).unwrap_or(());

    self.unity.replace(unity);

    Ok(())
  }

  pub fn set_progress(&mut self, progress: f64) -> Result<(), Box<dyn std::error::Error>> {
    self.set_progress_state(true)?;

    self.progress = progress;

    if let Some(ref mut unity) = self.unity {
      unity.set_progress(progress)?;
    }
    Ok(())
  }

  pub fn set_progress_state(&mut self, visible: bool) -> Result<(), Box<dyn std::error::Error>> {
    self.progress_visible = visible;

    if let Some(ref mut unity) = self.unity {
      unity.set_progress_visible(visible)?;
    }
    Ok(())
  }

  pub fn needs_attention(
    &mut self,
    needs_attention: bool,
  ) -> Result<(), Box<dyn std::error::Error>> {
    self.needs_attention = needs_attention;

    if let Some(ref mut unity) = self.unity {
      unity.needs_attention(needs_attention)?;
    }
    Ok(())
  }
}
