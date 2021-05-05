use crate::menu::MenuItem;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Statusbar {
  pub icon: Vec<u8>,
  pub items: Vec<MenuItem>,
}

impl Statusbar {
  pub fn new(icon: PathBuf, items: Vec<MenuItem>) -> Self {
    let icon = std::fs::read(icon).expect("Unable to read icon file");
    Self { icon, items }
  }
}
