use simple_logger::SimpleLogger;
use tao::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  menu::{Menu, MenuItem, MenuType},
  window::WindowBuilder,
};

fn main() {
  SimpleLogger::new().init().unwrap();
  let event_loop = EventLoop::new();

  let custom_change_menu = MenuItem::new("Change menu").with_accelerators("F1");

  let window = WindowBuilder::new()
    .with_title("A fantastic window!")
    .with_menu(vec![
      Menu::new(
        // on macOS first menu is always app name
        "my custom app",
        vec![
          MenuItem::About("Todos"),
          MenuItem::Services,
          MenuItem::Separator,
          MenuItem::Hide,
          MenuItem::HideOthers,
          MenuItem::ShowAll,
          MenuItem::Separator,
          MenuItem::Quit,
        ],
      ),
      Menu::new(
        "File",
        vec![
          custom_change_menu,
          MenuItem::Separator,
          MenuItem::CloseWindow,
        ],
      ),
      Menu::new(
        "Edit",
        vec![
          MenuItem::Undo,
          MenuItem::Redo,
          MenuItem::Separator,
          MenuItem::Cut,
          MenuItem::Copy,
          MenuItem::Paste,
          MenuItem::Separator,
          MenuItem::SelectAll,
        ],
      ),
      Menu::new("View", vec![MenuItem::EnterFullScreen]),
      Menu::new("Window", vec![MenuItem::Minimize, MenuItem::Zoom]),
      Menu::new(
        "Help",
        vec![MenuItem::new("Custom help")
          // `Primary` is a platform-agnostic accelerator modifier.
          // On Windows and Linux, `Primary` maps to the `Ctrl` key,
          // and on macOS it maps to the `command` key.
          .with_accelerators("<Primary><Shift>h")],
      ),
    ])
    .build(&event_loop)
    .unwrap();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        window_id,
      } if window_id == window.id() => *control_flow = ControlFlow::Exit,
      Event::MainEventsCleared => {
        window.request_redraw();
      }
      // not sure if we should add a new event type?
      // or try to re-use the UserEvent::<T>
      Event::MenuEvent {
        menu_id,
        origin: MenuType::Menubar,
      } => {
        if menu_id == custom_change_menu.id() {
          println!("Clicked on custom change menu");
        }

        println!("Clicked on {:?}", menu_id);
        window.set_title("New window title!");
      }
      _ => (),
    }
  });
}
