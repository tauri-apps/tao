// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use cocoa::{
  appkit::{NSApp, NSApplication, NSButton, NSEventModifierFlags, NSMenu, NSMenuItem},
  base::{id, nil, selector},
  foundation::{NSAutoreleasePool, NSString},
};
use objc::{
  declare::ClassDecl,
  runtime::{Class, Object, Sel, NO, YES},
};
use std::sync::Once;

use crate::{
  event::Event,
  menu::{MenuId, MenuItem, MenuType},
  platform::macos::NativeImage,
};

use super::{app_state::AppState, event::EventWrapper};

static BLOCK_PTR: &str = "taoMenuItemBlockPtr";

pub(crate) struct KeyEquivalent<'a> {
  pub(crate) key: &'a str,
  pub(crate) masks: Option<NSEventModifierFlags>,
}

#[derive(Debug, Clone)]
pub struct Menu {
  pub menu: id,
}

unsafe impl Send for Menu {}
unsafe impl Sync for Menu {}

#[derive(Debug, Clone)]
pub struct CustomMenuItem(pub(crate) id);

impl CustomMenuItem {
  pub fn set_enabled(&mut self, is_enabled: bool) {
    unsafe {
      let status = match is_enabled {
        true => YES,
        false => NO,
      };
      let () = msg_send![self.0, setEnabled: status];
    }
  }
  pub fn set_title(&mut self, title: &str) {
    unsafe {
      let menu_title = NSString::alloc(nil).init_str(title);
      self.0.setTitle_(menu_title);
    }
  }
  pub fn set_selected(&mut self, is_selected: bool) {
    unsafe {
      let state = match is_selected {
        true => 1_isize,
        false => 0_isize,
      };
      let () = msg_send![self.0, setState: state];
    }
  }
  pub fn set_icon(&mut self, icon: NativeImage) {
    unsafe {
      let ns_image: id = icon.get_ns_image();
      let image_ref: id = msg_send![class!(NSImage), imageNamed: ns_image];
      let () = msg_send![self.0, setImage: image_ref];
    }
  }
}

impl Default for Menu {
  fn default() -> Self {
    Menu::new()
  }
}

impl Menu {
  pub fn new() -> Self {
    unsafe {
      let menu = NSMenu::alloc(nil).autorelease();
      let () = msg_send![menu, setAutoenablesItems: NO];
      Self { menu }
    }
  }
  pub fn new_popup_menu() -> Self {
    Self::new()
  }
  pub fn add_item(&mut self, item: MenuItem, menu_type: MenuType) -> Option<CustomMenuItem> {
    let menu_item = match item {
      MenuItem::Separator => {
        unsafe {
          let sep = id::separatorItem(self.menu);
          self.menu.addItem_(sep);
        }
        None
      }
      MenuItem::Submenu(title, enabled, menu) => {
        unsafe {
          let menu_title = NSString::alloc(nil).init_str(&title);
          let menu_item = NSMenuItem::alloc(nil).autorelease();
          let () = msg_send![menu.menu, setTitle: menu_title];
          let () = msg_send![menu_item, setTitle: menu_title];
          if !enabled {
            let () = msg_send![menu_item, setEnabled: NO];
          }
          menu_item.setSubmenu_(menu.menu);
          self.menu.addItem_(menu_item);
        }
        None
      }
      MenuItem::About(app_name) => {
        let title = format!("About {}", app_name);
        Some(make_menu_item(
          title.as_str(),
          Some(selector("orderFrontStandardAboutPanel:")),
          None,
          menu_type,
        ))
      }
      // Close window
      MenuItem::CloseWindow => Some(make_menu_item(
        "Close Window",
        Some(selector("performClose:")),
        Some(KeyEquivalent {
          key: "w",
          masks: None,
        }),
        menu_type,
      )),
      MenuItem::Quit => Some(make_menu_item(
        "Quit",
        Some(selector("terminate:")),
        Some(KeyEquivalent {
          key: "q",
          masks: None,
        }),
        menu_type,
      )),
      MenuItem::Hide => Some(make_menu_item(
        "Hide",
        Some(selector("hide:")),
        Some(KeyEquivalent {
          key: "h",
          masks: None,
        }),
        menu_type,
      )),
      MenuItem::HideOthers => Some(make_menu_item(
        "Hide Others",
        Some(selector("hideOtherApplications:")),
        Some(KeyEquivalent {
          key: "h",
          masks: Some(
            NSEventModifierFlags::NSAlternateKeyMask | NSEventModifierFlags::NSCommandKeyMask,
          ),
        }),
        menu_type,
      )),
      MenuItem::ShowAll => Some(make_menu_item(
        "Show All",
        Some(selector("unhideAllApplications:")),
        None,
        menu_type,
      )),
      MenuItem::EnterFullScreen => Some(make_menu_item(
        "Enter Full Screen",
        Some(selector("toggleFullScreen:")),
        Some(KeyEquivalent {
          key: "f",
          masks: Some(
            NSEventModifierFlags::NSCommandKeyMask | NSEventModifierFlags::NSControlKeyMask,
          ),
        }),
        menu_type,
      )),
      MenuItem::Minimize => Some(make_menu_item(
        "Minimize",
        Some(selector("performMiniaturize:")),
        Some(KeyEquivalent {
          key: "m",
          masks: None,
        }),
        menu_type,
      )),
      MenuItem::Zoom => Some(make_menu_item(
        "Zoom",
        Some(selector("performZoom:")),
        None,
        menu_type,
      )),
      MenuItem::Copy => Some(make_menu_item(
        "Copy",
        Some(selector("copy:")),
        Some(KeyEquivalent {
          key: "c",
          masks: None,
        }),
        menu_type,
      )),
      MenuItem::Cut => Some(make_menu_item(
        "Cut",
        Some(selector("cut:")),
        Some(KeyEquivalent {
          key: "x",
          masks: None,
        }),
        menu_type,
      )),
      MenuItem::Paste => Some(make_menu_item(
        "Paste",
        Some(selector("paste:")),
        Some(KeyEquivalent {
          key: "v",
          masks: None,
        }),
        menu_type,
      )),
      MenuItem::Undo => Some(make_menu_item(
        "Undo",
        Some(selector("undo:")),
        Some(KeyEquivalent {
          key: "z",
          masks: None,
        }),
        menu_type,
      )),
      MenuItem::Redo => Some(make_menu_item(
        "Redo",
        Some(selector("redo:")),
        Some(KeyEquivalent {
          key: "Z",
          masks: None,
        }),
        menu_type,
      )),
      MenuItem::SelectAll => Some(make_menu_item(
        "Select All",
        Some(selector("selectAll:")),
        Some(KeyEquivalent {
          key: "a",
          masks: None,
        }),
        menu_type,
      )),
      MenuItem::Services => unsafe {
        let item = make_menu_item("Services", None, None, MenuType::Menubar);
        let app_class = class!(NSApplication);
        let app: id = msg_send![app_class, sharedApplication];
        let services: id = msg_send![app, servicesMenu];
        let _: () = msg_send![&*item, setSubmenu: services];
        Some(item)
      },
      MenuItem::Custom(custom_menu_item) => Some(custom_menu_item.0),
    };

    if let Some(menu_item) = menu_item {
      unsafe {
        self.menu.addItem_(menu_item);
      }

      return Some(CustomMenuItem(menu_item));
    }

    None
  }

  pub fn add_custom_item(
    &mut self,
    id: MenuId,
    menu_type: MenuType,
    text: &str,
    key: Option<&str>,
    enabled: bool,
    selected: bool,
  ) -> CustomMenuItem {
    let mut key_equivalent = None;
    let mut accelerator_string: String;
    if let Some(accelerator) = key {
      accelerator_string = accelerator.to_string();
      let mut ns_modifier_flags: NSEventModifierFlags = NSEventModifierFlags::empty();
      if accelerator_string.contains("<Primary>") {
        accelerator_string = accelerator_string.replace("<Primary>", "");
        ns_modifier_flags.insert(NSEventModifierFlags::NSCommandKeyMask);
      }

      if accelerator_string.contains("<Shift>") {
        accelerator_string = accelerator_string.replace("<Shift>", "");
        ns_modifier_flags.insert(NSEventModifierFlags::NSShiftKeyMask);
      }

      if accelerator_string.contains("<Ctrl>") {
        accelerator_string = accelerator_string.replace("<Ctrl>", "");
        ns_modifier_flags.insert(NSEventModifierFlags::NSControlKeyMask);
      }

      let mut masks = None;
      if !ns_modifier_flags.is_empty() {
        masks = Some(ns_modifier_flags);
      }

      key_equivalent = Some(KeyEquivalent {
        key: accelerator_string.as_str(),
        masks,
      });
    }

    let menu_item = make_custom_menu_item(id, &text, None, key_equivalent, menu_type);

    unsafe {
      self.menu.addItem_(menu_item);
      if selected {
        let () = msg_send![menu_item, setState: 1_isize];
      }
      if !enabled {
        let () = msg_send![menu_item, setEnabled: NO];
      }
    }

    CustomMenuItem(menu_item)
  }
}

#[derive(Debug)]
struct Action(Box<u32>);

pub fn initialize(menu_builder: Menu) {
  unsafe {
    let app = NSApp();
    app.setMainMenu_(menu_builder.menu);
  }
}

fn make_menu_alloc() -> *mut Object {
  unsafe { msg_send![make_menu_item_class(), alloc] }
}

pub(crate) fn make_custom_menu_item(
  id: MenuId,
  title: &str,
  selector: Option<Sel>,
  key_equivalent: Option<KeyEquivalent<'_>>,
  menu_type: MenuType,
) -> *mut Object {
  let alloc = make_menu_alloc();
  let menu_id = Box::new(Action(Box::new(id.0)));
  let ptr = Box::into_raw(menu_id);

  unsafe {
    (&mut *alloc).set_ivar(BLOCK_PTR, ptr as usize);
    let _: () = msg_send![&*alloc, setTarget:&*alloc];
    let title = NSString::alloc(nil).init_str(title);
    make_menu_item_from_alloc(alloc, title, selector, key_equivalent, menu_type)
  }
}

pub(crate) fn make_menu_item(
  title: &str,
  selector: Option<Sel>,
  key_equivalent: Option<KeyEquivalent<'_>>,
  menu_type: MenuType,
) -> *mut Object {
  let alloc = make_menu_alloc();
  unsafe {
    let title = NSString::alloc(nil).init_str(title);
    make_menu_item_from_alloc(alloc, title, selector, key_equivalent, menu_type)
  }
}

fn make_menu_item_from_alloc(
  alloc: *mut Object,
  title: *mut Object,
  selector: Option<Sel>,
  key_equivalent: Option<KeyEquivalent<'_>>,
  menu_type: MenuType,
) -> *mut Object {
  unsafe {
    let (key, masks) = match key_equivalent {
      Some(ke) => (NSString::alloc(nil).init_str(ke.key), ke.masks),
      None => (NSString::alloc(nil).init_str(""), None),
    };
    // if no selector defined, that mean it's a custom
    // menu so fire our handler
    let selector = match selector {
      Some(selector) => selector,
      None => match menu_type {
        MenuType::Menubar => sel!(fireMenubarAction:),
        MenuType::SystemTray => sel!(fireStatusbarAction:),
      },
    };

    // allocate our item to our class
    let item: id = msg_send![alloc, initWithTitle: title action: selector keyEquivalent: key];
    if let Some(masks) = masks {
      item.setKeyEquivalentModifierMask_(masks)
    }

    item
  }
}

fn make_menu_item_class() -> *const Class {
  static mut APP_CLASS: *const Class = 0 as *const Class;
  static INIT: Once = Once::new();

  INIT.call_once(|| unsafe {
    let superclass = class!(NSMenuItem);
    let mut decl = ClassDecl::new("TaoMenuItem", superclass).unwrap();
    decl.add_ivar::<usize>(BLOCK_PTR);

    decl.add_method(
      sel!(dealloc),
      dealloc_custom_menuitem as extern "C" fn(&Object, _),
    );

    decl.add_method(
      sel!(fireMenubarAction:),
      fire_menu_bar_click as extern "C" fn(&Object, _, id),
    );

    decl.add_method(
      sel!(fireStatusbarAction:),
      fire_status_bar_click as extern "C" fn(&Object, _, id),
    );

    APP_CLASS = decl.register();
  });

  unsafe { APP_CLASS }
}

extern "C" fn fire_status_bar_click(this: &Object, _: Sel, _item: id) {
  send_event(this, MenuType::SystemTray);
}

extern "C" fn fire_menu_bar_click(this: &Object, _: Sel, _item: id) {
  send_event(this, MenuType::Menubar);
}

fn send_event(this: &Object, origin: MenuType) {
  let menu_id = unsafe {
    let ptr: usize = *this.get_ivar(BLOCK_PTR);
    let obj = ptr as *const Action;
    &*obj
  };
  let event = Event::MenuEvent {
    menu_id: MenuId(*menu_id.0),
    origin,
  };
  AppState::queue_event(EventWrapper::StaticEvent(event));
}

extern "C" fn dealloc_custom_menuitem(this: &Object, _: Sel) {
  unsafe {
    let ptr: usize = *this.get_ivar(BLOCK_PTR);
    let obj = ptr as *mut Action;
    if !obj.is_null() {
      let _handler = Box::from_raw(obj);
    }
    let _: () = msg_send![super(this, class!(NSMenuItem)), dealloc];
  }
}
