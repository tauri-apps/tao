use zbus::{blocking::Connection, MessageBuilder};

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

enum DataTypes<'a> {
  Str(&'a str),
  String(&'a String),
  Bool(&'a bool),
  Number(&'a i32),
  Float(&'a f64),
}

impl Manager {
  pub fn new(app_uri: String) -> Result<Self, zbus::Error> {
    let conn = Connection::session()?;
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
    let mut properties: Vec<DataTypes> = Vec::new();

    properties.push(DataTypes::String(&self.app_uri));

    if self.dirty_progress {
      self.dirty_progress = false;

      properties.push(DataTypes::Str("progress"));

      properties.push(DataTypes::Float(&self.progress));
    }
    if self.dirty_count {
      self.dirty_count = false;

      properties.push(DataTypes::Str("count"));
      properties.push(DataTypes::Number(&self.count));
    }
    if self.dirty_progress_visible {
      self.dirty_progress_visible = false;

      properties.push(DataTypes::Str("progress-visible"));
      properties.push(DataTypes::Bool(&self.progress_visible));
    }
    if self.dirty_count_visible {
      self.dirty_count_visible = false;

      properties.push(DataTypes::Str("count-visible"));
      properties.push(DataTypes::Bool(&self.count_visible));
    }
    if self.dirty_urgent {
      self.dirty_urgent = false;

      properties.push(DataTypes::Str("urgent"));
      properties.push(DataTypes::Bool(&self.urgent));
    }

    if !properties.is_empty() {
      let mapped = properties
        .iter()
        .map(|x| match x {
          DataTypes::Str(string) => string.clone().to_string(),
          DataTypes::String(string) => string.clone().to_owned(),
          DataTypes::Bool(val) => val.to_string(),
          DataTypes::Number(val) => val.to_string(),
          DataTypes::Float(val) => val.to_string(),
        })
        .collect::<Vec<String>>();

      let signal = MessageBuilder::signal("/", "com.canonical.Unity.LauncherEntry", "Update")?
        .build(&mapped)?;

      self.conn.send_message(signal).unwrap();
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
