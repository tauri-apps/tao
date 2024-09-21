use gtk::{
  prelude::*, 
  HeaderBar, 
  Button, 
  Image, 
  ApplicationWindow, 
  EventBox, 
  Orientation, 
  gdk::EventMask,
  glib::{self},
};
use std::cell::RefCell;
use std::rc::Rc;

pub struct WlHeader;

impl WlHeader {
  pub fn setup(window: &ApplicationWindow) {
      let header = HeaderBar::new();
      header.set_show_close_button(false);

      let close_button = Self::create_header_button("window-close-symbolic");
      let maximize_button = Self::create_header_button("window-maximize-symbolic");
      let minimize_button = Self::create_header_button("window-minimize-symbolic");

      Self::connect_close_button(close_button.clone(), window);
      Self::connect_maximize_button(maximize_button.clone(), window);
      Self::connect_minimize_button(minimize_button.clone(), window);

      header.pack_end(&close_button);
      header.pack_end(&maximize_button);
      header.pack_end(&minimize_button);

      let drag_area = EventBox::new();
      drag_area.add(&header);
      drag_area.add_events(EventMask::BUTTON_PRESS_MASK | EventMask::BUTTON_RELEASE_MASK | EventMask::POINTER_MOTION_MASK);

      Self::connect_drag_area(&drag_area, window);

      let vbox = gtk::Box::new(Orientation::Vertical, 0);
      vbox.pack_start(&drag_area, false, false, 0);

      window.add(&vbox);
      window.set_titlebar(Some(&vbox));
  }

  fn create_header_button(icon_name: &str) -> Button {
      let button = Button::new();
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

  fn connect_maximize_button(button: Button, window: &ApplicationWindow) {
      let window_weak = window.downgrade();
      button.connect_clicked(move |_| {
          if let Some(window) = window_weak.upgrade() {
              if window.is_maximized() {
                  window.unmaximize();
              } else {
                  window.set_resizable(true);
                  window.maximize();
              }
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

  fn connect_drag_area(drag_area: &EventBox, window: &ApplicationWindow) {
      let window_weak = Rc::new(window.downgrade());
      let press_pos = Rc::new(RefCell::new((0, 0)));
      let state = Rc::new(RefCell::new((false, false))); // (moving, resizing)
  
      let press_pos_clone = press_pos.clone();
      let state_clone = state.clone();
      let window_weak_clone = window_weak.clone();
      drag_area.connect_button_press_event(move |_, event| {
          if let Some(_window) = window_weak_clone.upgrade() {
              let mut state = state_clone.borrow_mut();
              if event.button() == 1 { // 좌클릭
                  *press_pos_clone.borrow_mut() = (event.position().0 as i32, event.position().1 as i32);
                  state.0 = true; // moving = true
              } else if event.button() == 3 { // 우클릭
                  state.1 = true; // resizing = true
              }
          }
          glib::Propagation::Proceed
      });
  
      let state_clone = state.clone();
      drag_area.connect_button_release_event(move |_, _| {
          let mut state = state_clone.borrow_mut();
          state.0 = false; // moving = false
          state.1 = false; // resizing = false
          glib::Propagation::Proceed
      });
  
      let press_pos_clone = press_pos.clone();
      let state_clone = state.clone();
      let window_weak_clone = window_weak.clone();
      drag_area.connect_motion_notify_event(move |_, event| {
          if let Some(window) = window_weak_clone.upgrade() {
              let state = state_clone.borrow();
              let press_pos = press_pos_clone.borrow();
              if state.0 { // if moving
                  let (win_x, win_y) = window.position();
                  window.move_(
                      win_x + (event.position().0 as i32 - press_pos.0),
                      win_y + (event.position().1 as i32 - press_pos.1)
                  );
              } else if state.1 { // if resizing
                  let (width, height) = window.size();
                  window.resize(
                      width + (event.position().0 as i32 - press_pos.0),
                      height + (event.position().1 as i32 - press_pos.1)
                  );
              }
          }
          glib::Propagation::Proceed
      });
  }

}