use gtk::{
  glib::{self},
  prelude::*,
  ApplicationWindow, EventBox, HeaderBar,
};

pub struct WlHeader;

impl WlHeader {
  pub fn setup(window: &ApplicationWindow, title: &str) {
    let header = HeaderBar::builder()
      .show_close_button(true)
      .decoration_layout("menu:minimize,maximize,close")
      .title(title)
      .build();

    let event_box = EventBox::new();
    event_box.set_above_child(true);
    event_box.set_visible(true);
    event_box.set_can_focus(false);
    event_box.add(&header);

    window.set_titlebar(Some(&event_box));
    Self::connect_move_window(&event_box, &window);
  }

  fn connect_move_window(event_box: &EventBox, window: &ApplicationWindow) {
    let window_weak = window.downgrade();
    event_box.connect_button_press_event(move |_, event| {
      const LMB: u32 = 1;
      if event.button() == LMB {
        if let Some(window) = window_weak.upgrade() {
          let (x, y) = event.root();
          window.begin_move_drag(LMB as i32, x as i32, y as i32, event.time());
          return glib::Propagation::Stop;
        }
      }
      glib::Propagation::Proceed
    });
  }
}
