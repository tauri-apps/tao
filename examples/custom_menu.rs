use simple_logger::SimpleLogger;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    menu::{Menu, MenuItem},
    window::WindowBuilder,
};

pub enum Message {
    MenuClickAddNewItem,
}

fn main() {
    SimpleLogger::new().init().unwrap();
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_menu(vec![
            Menu::new(
                // on macOS first menu is always app name
                "my custom app",
                vec![
                    MenuItem::About("Todos".to_string()),
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
                    MenuItem::new("change_menu".to_string(), "Change menu".to_string()).key("+"),
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
                vec![MenuItem::new("help_me".to_string(), "Custom help".to_string()).key("-")],
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
            Event::MenuEvent(menu_id) => {
                if menu_id == "change_menu" {
                    // update menu
                    window.set_menu(Some(vec![Menu::new(
                        "File",
                        vec![
                            MenuItem::new("add_todo".to_string(), "Add Todo".to_string()).key("+"),
                            MenuItem::Separator,
                            MenuItem::CloseWindow,
                        ],
                    )]))
                }

                println!("Clicked on {}", menu_id);
                window.set_title("New window title!");
            }
            _ => (),
        }
    });
}
