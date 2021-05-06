use simple_logger::SimpleLogger;
use std::{collections::HashMap, path::PathBuf};
use tao::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  menu::{MenuItem, MenuType},
  status_bar::StatusbarBuilder,
  window::Window,
};

fn main() {
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();
  let mut windows = HashMap::new();

  let open_new_window = MenuItem::new("Open new window");

  // Windows always need his special touch!
  #[cfg(target_os = "windows")]
  let icon = PathBuf::from("examples/icon.ico");
  #[cfg(not(target_os = "windows"))]
  let icon = PathBuf::from("examples/icon.png");

  let _statusbar = StatusbarBuilder::new(icon, vec![open_new_window])
    .unwrap()
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, event_loop, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent { event, window_id } => {
        if event == WindowEvent::CloseRequested {
          println!("Window {:?} has received the signal to close", window_id);

          // Remove window from our hashmap
          windows.remove(&window_id);
        }
      }
      Event::MenuEvent {
        menu_id,
        origin: MenuType::Statusbar,
      } => {
        if menu_id == open_new_window.id() {
          let window = Window::new(&event_loop).unwrap();
          windows.insert(window.id(), window);
        }
        println!("Clicked on {:?}", menu_id);
      }
      _ => (),
    }
  });
}
