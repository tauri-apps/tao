use ashpd::{
  desktop::settings::{ColorScheme, Settings},
  Error,
};
use futures_lite::stream::StreamExt;
use gtk::glib::Sender;
use log::warn;

use crate::{
  platform_impl::{platform::window::WindowRequest, WindowId},
  window::Theme,
};

pub async fn theme() -> Result<Theme, Error> {
  let proxy = Settings::new().await?;
  Ok(proxy.color_scheme().await?.into())
}

pub async fn receive_theme_changed(tx: Sender<(WindowId, WindowRequest)>) -> Result<(), Error> {
  let proxy = Settings::new().await?;
  let mut stream = proxy.receive_color_scheme_changed().await?;

  while let Some(color_scheme) = stream.next().await {
    if let Err(e) = tx.send((
      WindowId::dummy(),
      WindowRequest::SetTheme(Some(color_scheme.into())),
    )) {
      warn!("Failed to send window request to request channel: {}", e);
    }
  }

  Ok(())
}

impl From<ColorScheme> for Theme {
  fn from(color_scheme: ColorScheme) -> Self {
    match color_scheme {
      ColorScheme::PreferDark => Theme::Dark,
      ColorScheme::PreferLight => Theme::Light,
      ColorScheme::NoPreference => Theme::Light,
    }
  }
}
