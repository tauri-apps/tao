use dbus::{
  arg::{PropMap, Variant},
  blocking::Connection,
  channel::Sender,
  strings::{Interface, Member},
  Message, Path,
};

pub struct Manager {
  conn: Connection,
  app_uri: String,
  count: i32,
  progress: f64,
  progress_visible: bool,
  count_visible: bool,
  urgent: bool,
  dirty_count: bool,
  dirty_progress: bool,
  dirty_progress_visible: bool,
  dirty_count_visible: bool,
  dirty_urgent: bool,
}

impl Manager {
  pub fn new(app_uri: String) -> Result<Self, dbus::Error> {
    let conn = Connection::new_session()?;
    let mut m = Self {
      conn,
      app_uri,
      progress: 0.0,
      count: 0,
      progress_visible: false,
      count_visible: false,
      urgent: false,
      dirty_count: true,
      dirty_progress: true,
      dirty_progress_visible: true,
      dirty_count_visible: true,
      dirty_urgent: true,
    };

    m.update().unwrap_or(());
    Ok(m)
  }

  fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    let mut properties = PropMap::new();
    if self.dirty_progress {
      self.dirty_progress = false;
      properties.insert("progress".to_owned(), Variant(Box::new(self.progress)));
    }
    if self.dirty_count {
      self.dirty_count = false;
      properties.insert("count".to_owned(), Variant(Box::new(self.count)));
    }
    if self.dirty_progress_visible {
      self.dirty_progress_visible = false;
      properties.insert(
        "progress-visible".to_owned(),
        Variant(Box::new(self.progress_visible)),
      );
    }
    if self.dirty_count_visible {
      self.dirty_count_visible = false;
      properties.insert(
        "count-visible".to_owned(),
        Variant(Box::new(self.count_visible)),
      );
    }
    if self.dirty_urgent {
      self.dirty_urgent = false;
      properties.insert("urgent".to_owned(), Variant(Box::new(self.urgent)));
    }
    if !properties.is_empty() {
      let signal = Message::signal(
        &Path::new("/")?,
        &Interface::new("com.canonical.Unity.LauncherEntry")?,
        &Member::new("Update")?,
      )
      .append1(&self.app_uri)
      .append1(properties);
      self.conn.send(signal).unwrap();
    }
    Ok(())
  }

  pub fn set_progress(&mut self, progress: f64) -> Result<(), Box<dyn std::error::Error>> {
    self.progress = progress;
    self.dirty_progress = true;
    self.update()?;
    Ok(())
  }

  pub fn set_progress_visible(
    &mut self,
    is_visible: bool,
  ) -> Result<(), Box<dyn std::error::Error>> {
    self.progress_visible = is_visible;
    self.dirty_progress_visible = true;
    self.update()?;
    Ok(())
  }

  pub fn needs_attention(
    &mut self,
    needs_attention: bool,
  ) -> Result<(), Box<dyn std::error::Error>> {
    self.urgent = needs_attention;
    self.dirty_urgent = true;
    self.update()?;
    Ok(())
  }
}
