use zbus::{
  blocking::Connection,
  fdo::Result,
  zvariant::{DeserializeDict, SerializeDict, Type},
  MessageBuilder,
};

pub struct Manager {
  conn: Connection,
  app_uri: String,
  progress: f64,
  progress_visible: bool,
  urgent: bool,
  dirty_progress: bool,
  dirty_progress_visible: bool,
  dirty_urgent: bool,
}

#[derive(Default, SerializeDict, DeserializeDict, Type, PartialEq, Debug)]
#[zvariant(signature = "dict")]
struct Progress {
  progress: Option<f64>,
  #[zvariant(rename = "progress-visible")]
  progress_visible: Option<bool>,
  urgent: Option<bool>,
}

impl Manager {
  pub fn new(app_uri: String) -> Result<Self> {
    let conn = Connection::session()?;
    let mut m = Self {
      conn,
      app_uri,
      progress: 0.0,
      progress_visible: false,
      urgent: false,
      dirty_progress: true,
      dirty_progress_visible: true,
      dirty_urgent: true,
    };

    m.update().unwrap_or(());
    Ok(m)
  }

  fn update(&mut self) -> Result<()> {
    let mut properties = Progress::default();

    if self.dirty_progress {
      self.dirty_progress = false;

      properties.progress = Some(*&self.progress);
    }
    if self.dirty_progress_visible {
      self.dirty_progress_visible = false;

      properties.progress_visible = Some(*&self.progress_visible);
    }
    if self.dirty_urgent {
      self.dirty_urgent = false;

      properties.urgent = Some(*&self.urgent);
    }

    let signal = MessageBuilder::signal("/", "com.canonical.Unity.LauncherEntry", "Update")?
      .build(&(self.app_uri.clone(), properties))?;

    self.conn.send_message(signal).unwrap();
    Ok(())
  }

  pub fn set_progress(&mut self, progress: f64) -> Result<()> {
    self.progress = progress;
    self.dirty_progress = true;
    self.update()?;
    Ok(())
  }

  pub fn set_progress_visible(&mut self, is_visible: bool) -> Result<()> {
    self.progress_visible = is_visible;
    self.dirty_progress_visible = true;
    self.update()?;
    Ok(())
  }

  pub fn needs_attention(&mut self, needs_attention: bool) -> Result<()> {
    self.urgent = needs_attention;
    self.dirty_urgent = true;
    self.update()?;
    Ok(())
  }
}
