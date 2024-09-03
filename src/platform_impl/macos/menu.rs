// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use cocoa::{
  appkit::{NSApp, NSApplication, NSButton, NSEventModifierFlags, NSImage, NSMenu, NSMenuItem},
  base::{id, nil, selector},
  foundation::{NSArray, NSAutoreleasePool, NSData, NSDictionary, NSSize, NSString, NSUInteger},
};
use objc::{
  declare::ClassDecl,
  runtime::{Class, Object, Sel, NO, YES},
};
use std::sync::Once;

use crate::{
  accelerator::{Accelerator, RawMods},
  event::Event,
  icon::Icon,
  keyboard::{KeyCode, ModifiersState},
  menu::{CustomMenuItem, MenuId, MenuItem, MenuType, SharingItem},
  platform::macos::NativeImage,
  window::WindowId,
};

use super::{
  app_state::AppState,
  event::EventWrapper,
  util::{app_name_string, ns_string_to_rust},
  window::get_window_id,
};

static BLOCK_PTR: &str = "taoMenuItemBlockPtr";

#[derive(Debug, Clone)]
pub struct Menu {
  pub menu: id,
}

unsafe impl Send for Menu {}
unsafe impl Sync for Menu {}

#[derive(Debug, Clone)]
pub struct MenuItemAttributes(Option<MenuId>, pub(crate) id);

impl MenuItemAttributes {
  pub fn id(self) -> MenuId {
    if let Some(menu_id) = self.0 {
      return menu_id;
    }
    // return empty menu value
    // can be used to compare
    MenuId::EMPTY
  }
  pub fn title(&self) -> String {
    unsafe {
      let title: id = msg_send![self.1, title];
      ns_string_to_rust(title)
    }
  }

  pub fn set_enabled(&mut self, is_enabled: bool) {
    unsafe {
      let status = match is_enabled {
        true => YES,
        false => NO,
      };
      let () = msg_send![self.1, setEnabled: status];
    }
  }
  pub fn set_title(&mut self, title: &str) {
    let title = super::util::strip_mnemonic(title);
    unsafe {
      let menu_title = NSString::alloc(nil).init_str(&title);
      self.1.setTitle_(menu_title);
    }
  }
  pub fn set_selected(&mut self, is_selected: bool) {
    unsafe {
      let state = match is_selected {
        true => 1_isize,
        false => 0_isize,
      };
      let () = msg_send![self.1, setState: state];
    }
  }

  pub fn set_icon(&mut self, icon: Icon) {
    let (width, height) = icon.inner.get_size();
    if let Ok(icon) = icon.inner.to_png() {
      let icon_height: f64 = 18.0;
      let icon_width: f64 = (width as f64) / (height as f64 / icon_height);

      unsafe {
        let nsdata = NSData::dataWithBytes_length_(
          nil,
          icon.as_ptr() as *const std::os::raw::c_void,
          icon.len() as u64,
        );

        let nsimage = NSImage::initWithData_(NSImage::alloc(nil), nsdata);
        let new_size = NSSize::new(icon_width, icon_height);
        let _: () = msg_send![nsimage, setSize: new_size];
        let _: () = msg_send![self.1, setImage: nsimage];
      }
    }
  }

  // Available only with CustomMenuItemExtMacOS
  pub fn set_native_image(&mut self, icon: NativeImage) {
    unsafe {
      let named_img: id = icon.get_ns_image();
      let nsimage: id = msg_send![class!(NSImage), imageNamed: named_img];
      let size = NSSize::new(18.0, 18.0);
      let _: () = msg_send![nsimage, setSize: size];
      let _: () = msg_send![self.1, setImage: nsimage];
    }
  }
}

impl Default for Menu {
  fn default() -> Self {
    Menu::new()
  }
}

impl Drop for Menu {
  fn drop(&mut self) {
    unsafe {
      let _: () = msg_send![self.menu, release];
    }
  }
}

impl Menu {
  pub fn new() -> Self {
    unsafe {
      let menu = NSMenu::alloc(nil);
      let _: () = msg_send![menu, retain];
      let () = msg_send![menu, setAutoenablesItems: NO];
      Self { menu }
    }
  }
  pub fn new_popup_menu() -> Self {
    Self::new()
  }

  pub fn add_item(
    &mut self,
    menu_id: MenuId,
    title: &str,
    accelerators: Option<Accelerator>,
    enabled: bool,
    selected: bool,
    menu_type: MenuType,
  ) -> CustomMenuItem {
    let menu_item = make_custom_menu_item(menu_id, title, None, accelerators, menu_type);

    unsafe {
      if selected {
        let () = msg_send![menu_item, setState: 1_isize];
      }
      if !enabled {
        let () = msg_send![menu_item, setEnabled: NO];
      }

      self.menu.addItem_(menu_item);
    }

    CustomMenuItem(MenuItemAttributes(Some(menu_id), menu_item))
  }

  pub fn add_submenu(&mut self, title: &str, enabled: bool, submenu: Menu) {
    unsafe {
      let title = super::util::strip_mnemonic(title);
      let menu_title = NSString::alloc(nil).init_str(&title);
      let menu_item = NSMenuItem::alloc(nil).autorelease();
      let () = msg_send![submenu.menu, setTitle: menu_title];
      let () = msg_send![menu_item, setTitle: menu_title];
      if !enabled {
        let () = msg_send![menu_item, setEnabled: NO];
      }
      menu_item.setSubmenu_(submenu.menu);
      self.menu.addItem_(menu_item);
    }
  }

  pub fn add_native_item(&mut self, item: MenuItem, menu_type: MenuType) -> Option<CustomMenuItem> {
    let menu_details: Option<(Option<MenuId>, *mut Object)> = match item {
      MenuItem::Separator => {
        unsafe {
          let sep = id::separatorItem(self.menu);
          self.menu.addItem_(sep);
        }
        None
      }
      MenuItem::About(name, _) => {
        let title = format!("About {}", name);
        Some((
          None,
          make_menu_item(
            title.as_str(),
            Some(selector("orderFrontStandardAboutPanel:")),
            None,
            menu_type,
          ),
        ))
      }
      // Close window
      MenuItem::CloseWindow => Some((
        None,
        make_menu_item(
          "Close Window",
          Some(selector("performClose:")),
          Some(Accelerator::new(RawMods::Meta, KeyCode::KeyW)),
          menu_type,
        ),
      )),
      MenuItem::Quit => Some((
        None,
        make_menu_item(
          format!("Quit {}", unsafe { app_name_string() }.unwrap_or_default()).trim(),
          Some(selector("terminate:")),
          Some(Accelerator::new(RawMods::Meta, KeyCode::KeyQ)),
          menu_type,
        ),
      )),
      MenuItem::Hide => Some((
        None,
        make_menu_item(
          format!("Hide {}", unsafe { app_name_string() }.unwrap_or_default()).trim(),
          Some(selector("hide:")),
          Some(Accelerator::new(RawMods::Meta, KeyCode::KeyH)),
          menu_type,
        ),
      )),
      MenuItem::HideOthers => Some((
        None,
        make_menu_item(
          "Hide Others",
          Some(selector("hideOtherApplications:")),
          Some(Accelerator::new(RawMods::AltMeta, KeyCode::KeyH)),
          menu_type,
        ),
      )),
      MenuItem::ShowAll => Some((
        None,
        make_menu_item(
          "Show All",
          Some(selector("unhideAllApplications:")),
          None,
          menu_type,
        ),
      )),
      MenuItem::EnterFullScreen => Some((
        None,
        make_menu_item(
          "Toggle Full Screen",
          Some(selector("toggleFullScreen:")),
          Some(Accelerator::new(RawMods::CtrlMeta, KeyCode::KeyF)),
          menu_type,
        ),
      )),
      MenuItem::Minimize => Some((
        None,
        make_menu_item(
          "Minimize",
          Some(selector("performMiniaturize:")),
          Some(Accelerator::new(RawMods::Meta, KeyCode::KeyM)),
          menu_type,
        ),
      )),
      MenuItem::Zoom => Some((
        None,
        make_menu_item("Zoom", Some(selector("performZoom:")), None, menu_type),
      )),
      MenuItem::Copy => Some((
        None,
        make_menu_item(
          "Copy",
          Some(selector("copy:")),
          Some(Accelerator::new(RawMods::Meta, KeyCode::KeyC)),
          menu_type,
        ),
      )),
      MenuItem::Cut => Some((
        None,
        make_menu_item(
          "Cut",
          Some(selector("cut:")),
          Some(Accelerator::new(RawMods::Meta, KeyCode::KeyX)),
          menu_type,
        ),
      )),
      MenuItem::Paste => Some((
        None,
        make_menu_item(
          "Paste",
          Some(selector("paste:")),
          Some(Accelerator::new(RawMods::Meta, KeyCode::KeyV)),
          menu_type,
        ),
      )),
      MenuItem::Undo => Some((
        None,
        make_menu_item(
          "Undo",
          Some(selector("undo:")),
          Some(Accelerator::new(RawMods::Meta, KeyCode::KeyZ)),
          menu_type,
        ),
      )),
      MenuItem::Redo => Some((
        None,
        make_menu_item(
          "Redo",
          Some(selector("redo:")),
          Some(Accelerator::new(RawMods::MetaShift, KeyCode::KeyZ)),
          menu_type,
        ),
      )),
      MenuItem::SelectAll => Some((
        None,
        make_menu_item(
          "Select All",
          Some(selector("selectAll:")),
          Some(Accelerator::new(RawMods::Meta, KeyCode::KeyA)),
          menu_type,
        ),
      )),
      MenuItem::Services => unsafe {
        let item = make_menu_item("Services", None, None, MenuType::MenuBar);
        // we have to assign an empty menu as the app's services menu, and macOS will populate it
        let services_menu = NSMenu::alloc(nil).autorelease();
        let app_class = class!(NSApplication);
        let app: id = msg_send![app_class, sharedApplication];
        let () = msg_send![app, setServicesMenu: services_menu];
        let () = msg_send![&*item, setSubmenu: services_menu];
        Some((None, item))
      },
      MenuItem::Share(items) => unsafe {
        let menu_item: id = NSMenuItem::alloc(nil).autorelease();
        let () = msg_send![menu_item, setTitle: NSString::alloc(nil).init_str("Share")];

        let share_menu = NSMenu::alloc(nil).autorelease();
        let () = msg_send![share_menu, setAutoenablesItems: NO];

        let ns_items: id /* NSMutableArray */ = convert_sharing_item_to_ns(items);
        let ns_items_count: u64 = msg_send![ns_items, count];

        if ns_items_count == 0 {
          let () = msg_send![menu_item, setEnabled: NO];
        } else {
          let services: id /* NSArray */ = msg_send![class!(NSSharingService), sharingServicesForItems:ns_items];
          let service_count: NSUInteger = msg_send![services, count];
          for i in 0..service_count {
            let service: id = msg_send![services, objectAtIndex: i];
            let () = msg_send![share_menu, addItem: make_share_menu_item(service, ns_items)];
          }
          menu_item.setSubmenu_(share_menu);
        }
        Some((None, menu_item))
      },
    };

    if let Some((menu_id, menu_item)) = menu_details {
      unsafe {
        self.menu.addItem_(menu_item);
      }

      return Some(CustomMenuItem(MenuItemAttributes(menu_id, menu_item)));
    }

    None
  }
}

#[derive(Debug)]
struct Action(Box<u16>);

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
  accelerators: Option<Accelerator>,
  menu_type: MenuType,
) -> *mut Object {
  let title = super::util::strip_mnemonic(title);

  let alloc = make_menu_alloc();
  let menu_id = Box::new(Action(Box::new(id.0)));
  let ptr = Box::into_raw(menu_id);

  unsafe {
    (&mut *alloc).set_ivar(BLOCK_PTR, ptr as usize);
    let _: () = msg_send![&*alloc, setTarget:&*alloc];
    let title = NSString::alloc(nil).init_str(&title);
    make_menu_item_from_alloc(alloc, title, selector, accelerators, menu_type)
  }
}

pub(crate) fn make_menu_item(
  title: &str,
  selector: Option<Sel>,
  accelerator: Option<Accelerator>,
  menu_type: MenuType,
) -> *mut Object {
  let alloc = make_menu_alloc();
  unsafe {
    let title = NSString::alloc(nil).init_str(title);
    make_menu_item_from_alloc(alloc, title, selector, accelerator, menu_type)
  }
}

fn make_menu_item_from_alloc(
  alloc: *mut Object,
  title: *mut Object,
  selector: Option<Sel>,
  accelerator: Option<Accelerator>,
  menu_type: MenuType,
) -> *mut Object {
  unsafe {
    // build our Accelerator string
    let key_equivalent = accelerator
      .clone()
      .map(Accelerator::key_equivalent)
      .unwrap_or_else(|| "".into());
    let key_equivalent = NSString::alloc(nil).init_str(&key_equivalent);
    // if no selector defined, that mean it's a custom
    // menu so fire our handler
    let selector = match selector {
      Some(selector) => selector,
      None => match menu_type {
        MenuType::MenuBar => sel!(fireMenubarAction:),
        MenuType::ContextMenu => sel!(fireStatusbarAction:),
      },
    };
    // allocate our item to our class
    let item: id =
      msg_send![alloc, initWithTitle: title action: selector keyEquivalent: key_equivalent];

    let mask = accelerator
      .map(Accelerator::key_modifier_mask)
      .unwrap_or_else(NSEventModifierFlags::empty);
    item.setKeyEquivalentModifierMask_(mask);
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
  send_event(this, MenuType::ContextMenu);
}

extern "C" fn fire_menu_bar_click(this: &Object, _: Sel, _item: id) {
  send_event(this, MenuType::MenuBar);
}

fn send_event(this: &Object, origin: MenuType) {
  let menu_id = unsafe {
    let ptr: usize = *this.get_ivar(BLOCK_PTR);
    let obj = ptr as *const Action;
    &*obj
  };

  // active window
  let window_id = match origin {
    MenuType::MenuBar => unsafe {
      let app: id = msg_send![class!(NSApplication), sharedApplication];
      let window_id: id = msg_send![app, mainWindow];
      Some(WindowId(get_window_id(window_id)))
    },
    // system tray do not send WindowId
    MenuType::ContextMenu => None,
  };

  let event = Event::MenuEvent {
    window_id,
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

pub(crate) fn make_share_menu_item(service: id, items: id) -> *mut Object {
  unsafe {
    let menu_item: id /* NSMenuItem */ = msg_send![make_share_menu_item_class(), alloc];

    let title: id = msg_send![service, menuItemTitle];
    let image: id = msg_send![service, image];
    let menu_item: id = msg_send![menu_item, initWithTitle:title action:sel!(performShare:) keyEquivalent:NSString::alloc(nil).init_str("")];

    let keys: id = NSArray::arrayWithObjects(
      nil,
      &[
        NSString::alloc(nil).init_str("service"),
        NSString::alloc(nil).init_str("items"),
      ],
    );
    let values: id = NSArray::arrayWithObjects(nil, &[service, items]);
    let perform_obj: id =
      msg_send![class!(NSDictionary), dictionaryWithObjects:values forKeys:keys];

    let () = msg_send![menu_item, setTarget:menu_item];
    let () = msg_send![menu_item, setImage:image];
    let () = msg_send![menu_item, setRepresentedObject:perform_obj];

    menu_item
  }
}

fn make_share_menu_item_class() -> *const Class {
  static mut APP_CLASS: *const Class = 0 as *const Class;
  static INIT: Once = Once::new();

  INIT.call_once(|| unsafe {
    let superclass = class!(NSMenuItem);
    let mut decl = ClassDecl::new("TaoShareMenuItem", superclass).unwrap();

    decl.add_method(
      sel!(performShare:),
      perform_share as extern "C" fn(&Object, _, id),
    );

    APP_CLASS = decl.register();
  });

  unsafe { APP_CLASS }
}

extern "C" fn perform_share(_: &Object, _: Sel, sender: id) {
  unsafe {
    let perform_obj: id /* NSDictionary */ = msg_send![sender, representedObject];
    let service = perform_obj.objectForKey_(NSString::alloc(nil).init_str("service"));
    let items = perform_obj.objectForKey_(NSString::alloc(nil).init_str("items"));

    let () = msg_send![service, performWithItems:items];
  }
}

fn convert_sharing_item_to_ns(items: SharingItem) -> id /* NSMutableArray */ {
  unsafe {
    let ns_items: id /* NSMutableArray */ = msg_send![class!(NSMutableArray), array];

    if let Some(texts) = items.texts {
      for text in texts {
        let () = msg_send![ns_items, addObject: NSString::alloc(nil).init_str(&text)];
      }
    }
    if let Some(urls) = items.urls {
      for url in urls {
        let url: id = msg_send![class!(NSURL), URLWithString:NSString::alloc(nil).init_str(&url)];
        let () = msg_send![ns_items, addObject:url];
      }
    }
    if let Some(file_paths) = items.file_paths {
      for file_path in file_paths {
        let path: id = NSString::alloc(nil).init_str(&file_path.display().to_string());
        let url: id = msg_send![class!(NSURL), fileURLWithPath:path isDirectory:false];
        let () = msg_send![ns_items, addObject:url];
      }
    }

    ns_items
  }
}

impl Accelerator {
  /// Return the string value of this hotkey, for use with Cocoa `NSResponder`
  /// objects.
  ///
  /// Returns the empty string if no key equivalent is known.
  fn key_equivalent(self) -> String {
    match self.key {
      KeyCode::KeyA => "a".into(),
      KeyCode::KeyB => "b".into(),
      KeyCode::KeyC => "c".into(),
      KeyCode::KeyD => "d".into(),
      KeyCode::KeyE => "e".into(),
      KeyCode::KeyF => "f".into(),
      KeyCode::KeyG => "g".into(),
      KeyCode::KeyH => "h".into(),
      KeyCode::KeyI => "i".into(),
      KeyCode::KeyJ => "j".into(),
      KeyCode::KeyK => "k".into(),
      KeyCode::KeyL => "l".into(),
      KeyCode::KeyM => "m".into(),
      KeyCode::KeyN => "n".into(),
      KeyCode::KeyO => "o".into(),
      KeyCode::KeyP => "p".into(),
      KeyCode::KeyQ => "q".into(),
      KeyCode::KeyR => "r".into(),
      KeyCode::KeyS => "s".into(),
      KeyCode::KeyT => "t".into(),
      KeyCode::KeyU => "u".into(),
      KeyCode::KeyV => "v".into(),
      KeyCode::KeyW => "w".into(),
      KeyCode::KeyX => "x".into(),
      KeyCode::KeyY => "y".into(),
      KeyCode::KeyZ => "z".into(),
      KeyCode::Digit0 => "0".into(),
      KeyCode::Digit1 => "1".into(),
      KeyCode::Digit2 => "2".into(),
      KeyCode::Digit3 => "3".into(),
      KeyCode::Digit4 => "4".into(),
      KeyCode::Digit5 => "5".into(),
      KeyCode::Digit6 => "6".into(),
      KeyCode::Digit7 => "7".into(),
      KeyCode::Digit8 => "8".into(),
      KeyCode::Digit9 => "9".into(),
      KeyCode::Comma => ",".into(),
      KeyCode::Minus => "-".into(),
      KeyCode::Plus => "+".into(),
      KeyCode::Period => ".".into(),
      KeyCode::Space => "\u{0020}".into(),
      KeyCode::Equal => "=".into(),
      KeyCode::Semicolon => ";".into(),
      KeyCode::Slash => "/".into(),
      KeyCode::Backslash => "\\".into(),
      KeyCode::Quote => "\'".into(),
      KeyCode::Backquote => "`".into(),
      KeyCode::BracketLeft => "[".into(),
      KeyCode::BracketRight => "]".into(),
      KeyCode::Tab => "⇥".into(),
      KeyCode::Escape => "\u{001b}".into(),
      // from NSText.h
      KeyCode::Enter => "\u{0003}".into(),
      KeyCode::Backspace => "\u{0008}".into(),
      KeyCode::Delete => "\u{007f}".into(),
      // from NSEvent.h
      KeyCode::Insert => "\u{F727}".into(),
      KeyCode::Home => "\u{F729}".into(),
      KeyCode::End => "\u{F72B}".into(),
      KeyCode::PageUp => "\u{F72C}".into(),
      KeyCode::PageDown => "\u{F72D}".into(),
      KeyCode::PrintScreen => "\u{F72E}".into(),
      KeyCode::ScrollLock => "\u{F72F}".into(),
      KeyCode::ArrowUp => "\u{F700}".into(),
      KeyCode::ArrowDown => "\u{F701}".into(),
      KeyCode::ArrowLeft => "\u{F702}".into(),
      KeyCode::ArrowRight => "\u{F703}".into(),
      KeyCode::F1 => "\u{F704}".into(),
      KeyCode::F2 => "\u{F705}".into(),
      KeyCode::F3 => "\u{F706}".into(),
      KeyCode::F4 => "\u{F707}".into(),
      KeyCode::F5 => "\u{F708}".into(),
      KeyCode::F6 => "\u{F709}".into(),
      KeyCode::F7 => "\u{F70A}".into(),
      KeyCode::F8 => "\u{F70B}".into(),
      KeyCode::F9 => "\u{F70C}".into(),
      KeyCode::F10 => "\u{F70D}".into(),
      KeyCode::F11 => "\u{F70E}".into(),
      KeyCode::F12 => "\u{F70F}".into(),
      KeyCode::F13 => "\u{F710}".into(),
      KeyCode::F14 => "\u{F711}".into(),
      KeyCode::F15 => "\u{F712}".into(),
      KeyCode::F16 => "\u{F713}".into(),
      KeyCode::F17 => "\u{F714}".into(),
      KeyCode::F18 => "\u{F715}".into(),
      KeyCode::F19 => "\u{F716}".into(),
      KeyCode::F20 => "\u{F717}".into(),
      KeyCode::F21 => "\u{F718}".into(),
      KeyCode::F22 => "\u{F719}".into(),
      KeyCode::F23 => "\u{F71A}".into(),
      KeyCode::F24 => "\u{F71B}".into(),
      _ => {
        #[cfg(debug_assertions)]
        eprintln!("no key equivalent for {:?}", self);
        "".into()
      }
    }
  }

  fn key_modifier_mask(self) -> NSEventModifierFlags {
    let mods: ModifiersState = self.mods;
    let mut flags = NSEventModifierFlags::empty();
    if mods.shift_key() {
      flags.insert(NSEventModifierFlags::NSShiftKeyMask);
    }
    if mods.super_key() {
      flags.insert(NSEventModifierFlags::NSCommandKeyMask);
    }
    if mods.alt_key() {
      flags.insert(NSEventModifierFlags::NSAlternateKeyMask);
    }
    if mods.control_key() {
      flags.insert(NSEventModifierFlags::NSControlKeyMask);
    }
    flags
  }
}
