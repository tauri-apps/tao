use gtk::{
  glib::{self},
  prelude::*,
  ApplicationWindow, HeaderBar,
};

pub struct WlHeader;

impl WlHeader {
  pub fn setup(window: &ApplicationWindow) {
    let header = HeaderBar::builder()
      .show_close_button(true)
      .decoration_layout("menu:minimize,maximize,close")
      .build();

    window.set_titlebar(Some(&header));
    Self::connect_move_window(&header, &window);
  }

  fn connect_move_window(header: &HeaderBar, window: &ApplicationWindow) {
    let header_weak = header.downgrade();
    window.connect_button_press_event(move |window, event| {
      const LMB: u32 = 1;
      if event.button() == LMB {
        if let Some(header) = header_weak.upgrade() {
          let header_height = header.allocated_height();
          let (x, y) = event.root();
          let (window_x, window_y) = window.position();
          let (window_width, _) = window.size();

          if x >= window_x as f64
            && x <= (window_x + window_width) as f64
            && y >= window_y as f64
            && y <= (window_y + header_height) as f64
          {
            window.begin_move_drag(LMB as i32, x as i32, y as i32, event.time());
            return glib::Propagation::Stop;
          }
        }
      }
      glib::Propagation::Proceed
    });
  }
}
