use crate::window::{ProgressBarState, ProgressState};
use zbus::{
  blocking::Connection,
  fdo::Result,
  zvariant::{DeserializeDict, SerializeDict, Type},
  Message,
};

pub struct TaskbarIndicator {
  conn: Connection,
  app_uri: String,
}

#[derive(Default, SerializeDict, DeserializeDict, Type, PartialEq, Debug)]
#[zvariant(signature = "dict")]
struct Progress {
  progress: Option<f64>,
  #[zvariant(rename = "progress-visible")]
  progress_visible: Option<bool>,
  urgent: Option<bool>,
}

impl TaskbarIndicator {
  pub fn new() -> Result<Self> {
    let conn = Connection::session()?;

    Ok(Self {
      conn,
      app_uri: String::new(),
    })
  }

  pub fn update(&mut self, progress: ProgressBarState) -> Result<()> {
    let mut properties = Progress::default();

    if let Some(uri) = progress.unity_uri {
      self.app_uri = uri;
    }

    if let Some(progress) = progress.progress {
      let progress = if progress > 100 { 100 } else { progress };

      properties.progress = Some(progress as f64 / 100.0);
    }

    if let Some(state) = progress.state {
      properties.progress_visible = Some(!matches!(state, ProgressState::None));
    }

    let signal = Message::signal("/", "com.canonical.Unity.LauncherEntry", "Update")?
      .build(&(self.app_uri.clone(), properties))?;

    self.conn.send(&signal)?;
    Ok(())
  }
}
