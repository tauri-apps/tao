use gtk::{
  gdk::{self},
  glib::{self},
  prelude::*,
  ApplicationWindow, Button, HeaderBar, Image, ToggleButton,
};
pub struct WlHeader;

impl WlHeader {
  pub fn setup(window: &ApplicationWindow) {
    let header = HeaderBar::new();
    header.set_show_close_button(false);

    let close_button = Self::create_header_button("window-close-symbolic");
    let maximize_button = Self::create_header_toggle_button("window-maximize-symbolic");
    let minimize_button = Self::create_header_button("window-minimize-symbolic");

    Self::connect_close_button(close_button.clone(), window);
    Self::connect_maximize_button(maximize_button.clone(), window);
    Self::connect_minimize_button(minimize_button.clone(), window);

    header.pack_end(&close_button);
    header.pack_end(&maximize_button);
    header.pack_end(&minimize_button);

    window.set_titlebar(Some(&header));
  }

  fn create_header_button(icon_name: &str) -> Button {
    let button = Button::new();
    let icon = Image::from_icon_name(Some(icon_name), gtk::IconSize::Button);
    button.set_image(Some(&icon));
    button
  }

  fn create_header_toggle_button(icon_name: &str) -> ToggleButton {
    let button = ToggleButton::new();
    let icon = Image::from_icon_name(Some(icon_name), gtk::IconSize::Button);
    button.set_image(Some(&icon));
    button
  }

  fn connect_close_button(button: Button, window: &ApplicationWindow) {
    let window_weak = window.downgrade();
    button.connect_clicked(move |_| {
      if let Some(window) = window_weak.upgrade() {
        window.close();
      }
    });
  }

  fn connect_maximize_button(button: ToggleButton, window: &ApplicationWindow) {
    let window_weak = window.downgrade();
    // initial state
    if let Some(window) = window_weak.upgrade() {
      button.set_active(window.is_maximized());
      button.set_sensitive(window.is_resizable());
    }

    let button_weak = button.downgrade();
    window.connect_is_maximized_notify(move |window| {
      if let Some(button) = button_weak.upgrade() {
        button.set_active(window.is_maximized());
      }
    });

    let button_weak = button.downgrade();
    window.connect_resizable_notify(move |window| {
      if let Some(button) = button_weak.upgrade() {
        button.set_sensitive(window.is_resizable());
      }
    });

    //click event
    button.connect_toggled(move |button| {
      if let Some(window) = window_weak.upgrade() {
        if button.is_active() && window.is_resizable() {
          window.maximize();
        } else {
          window.unmaximize();
        }
      }
    });

    // Prevent space key activation in header
    button.connect_key_press_event(move |_, event_key| {
      if event_key.keyval() == gdk::keys::constants::space {
        glib::Propagation::Stop
      } else {
        glib::Propagation::Proceed
      }
    });
  }

  fn connect_minimize_button(button: Button, window: &ApplicationWindow) {
    let window_weak = window.downgrade();
    button.connect_clicked(move |_| {
      if let Some(window) = window_weak.upgrade() {
        window.iconify();
      }
    });
  }
}
