// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use super::{
  app_state::AppState,
  event::EventWrapper,
  menu::Menu,
  util::{bottom_left_to_top_left_for_cursor, bottom_left_to_top_left_for_tray},
};
use crate::{
  dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
  error::OsError,
  event::{Event, Rectangle, TrayEvent},
  event_loop::EventLoopWindowTarget,
  system_tray::{Icon, SystemTray as RootSystemTray},
  TrayId,
};
use cocoa::{
  appkit::{
    NSButton, NSEventMask, NSEventModifierFlags, NSEventType, NSImage, NSStatusBar, NSStatusItem,
    NSVariableStatusItemLength, NSWindow,
  },
  base::{id, nil, NO, YES},
  foundation::{NSData, NSPoint, NSSize, NSString},
};
use objc::{
  declare::ClassDecl,
  runtime::{Class, Object, Protocol, Sel},
};
use std::sync::Once;

pub struct SystemTrayBuilder {
  pub(crate) system_tray: SystemTray,
}

impl SystemTrayBuilder {
  /// Creates a new SystemTray for platforms where this is appropriate.
  #[inline]
  pub fn new(icon: Icon, tray_menu: Option<Menu>) -> Self {
    unsafe {
      let ns_status_bar =
        NSStatusBar::systemStatusBar(nil).statusItemWithLength_(NSVariableStatusItemLength);
      let _: () = msg_send![ns_status_bar, retain];

      Self {
        system_tray: SystemTray {
          icon_is_template: false,
          icon,
          menu_on_left_click: true,
          tray_menu,
          ns_status_bar,
          title: None,
        },
      }
    }
  }

  /// Builds the system tray.
  #[inline]
  pub fn build<T: 'static>(
    self,
    _window_target: &EventLoopWindowTarget<T>,
    tray_id: TrayId,
    tooltip: Option<String>,
  ) -> Result<RootSystemTray, OsError> {
    unsafe {
      // use our existing status bar
      let status_bar = self.system_tray.ns_status_bar;

      // set our icon
      self.system_tray.create_button_with_icon();

      // attach click event to our button
      let button = status_bar.button();
      let tray_target: id = msg_send![make_tray_class(), alloc];
      let tray_target: id = msg_send![tray_target, init];
      (*tray_target).set_ivar("id", tray_id.0);
      (*tray_target).set_ivar("status_bar", status_bar);
      (*tray_target).set_ivar("menu", nil);
      (*tray_target).set_ivar("menu_on_left_click", self.system_tray.menu_on_left_click);
      let _: () = msg_send![button, setAction: sel!(click:)];
      let _: () = msg_send![button, setTarget: tray_target];
      let _: () = msg_send![
        button,
        sendActionOn: NSEventMask::NSLeftMouseDownMask
          | NSEventMask::NSRightMouseDownMask
          | NSEventMask::NSKeyDownMask
      ];

      // attach menu only if provided
      if let Some(menu) = self.system_tray.tray_menu.clone() {
        // We set the tray menu to tray_target instead of status bar
        // Because setting directly to status bar will overwrite the event callback of the button
        // See `make_tray_class` for more information.
        (*tray_target).set_ivar("menu", menu.menu);
        let () = msg_send![menu.menu, setDelegate: tray_target];
      }

      // attach tool_tip if provided
      if let Some(tooltip) = tooltip {
        self.system_tray.set_tooltip(&tooltip);
      }

      // set up title if provided
      if let Some(title) = &self.system_tray.title {
        self.system_tray.set_title(title);
      }
    }

    Ok(RootSystemTray(self.system_tray))
  }
}

/// System tray is a status icon that can show popup menu. It is usually displayed on top right or bottom right of the screen.
#[derive(Debug, Clone)]
pub struct SystemTray {
  pub(crate) icon: Icon,
  pub(crate) icon_is_template: bool,
  pub(crate) menu_on_left_click: bool,
  pub(crate) tray_menu: Option<Menu>,
  pub(crate) ns_status_bar: id,
  pub(crate) title: Option<String>,
}

impl Drop for SystemTray {
  fn drop(&mut self) {
    unsafe {
      NSStatusBar::systemStatusBar(nil).removeStatusItem_(self.ns_status_bar);
      let _: () = msg_send![self.ns_status_bar, release];
    }
  }
}

impl SystemTray {
  pub fn set_icon(&mut self, icon: Icon) {
    // update our icon
    self.icon = icon;
    self.create_button_with_icon();
  }

  pub fn set_icon_as_template(mut self, is_template: bool) {
    self.icon_is_template = is_template;
  }

  pub fn set_menu(&mut self, tray_menu: &Menu) {
    unsafe {
      self.ns_status_bar.setMenu_(tray_menu.menu);
    }
  }

  pub fn set_tooltip(&self, tooltip: &str) {
    unsafe {
      let tooltip = NSString::alloc(nil).init_str(tooltip);
      let _: () = msg_send![self.ns_status_bar.button(), setToolTip: tooltip];
    }
  }

  pub fn set_title(&self, title: &str) {
    unsafe {
      NSButton::setTitle_(
        self.ns_status_bar.button(),
        NSString::alloc(nil).init_str(title),
      );
    }
  }

  fn create_button_with_icon(&self) {
    // The image is to the right of the title https://developer.apple.com/documentation/appkit/nscellimageposition/nsimageleft
    const NSIMAGE_LEFT: i32 = 2;

    let icon = self.icon.inner.to_png();

    let (width, height) = self.icon.inner.get_size();

    let icon_height: f64 = 18.0;
    let icon_width: f64 = (width as f64) / (height as f64 / icon_height);

    unsafe {
      let status_item = self.ns_status_bar;
      let button = status_item.button();

      // build our icon
      let nsdata = NSData::dataWithBytes_length_(
        nil,
        icon.as_ptr() as *const std::os::raw::c_void,
        icon.len() as u64,
      );

      let nsimage = NSImage::initWithData_(NSImage::alloc(nil), nsdata);
      let new_size = NSSize::new(icon_width, icon_height);

      button.setImage_(nsimage);
      let _: () = msg_send![nsimage, setSize: new_size];
      let _: () = msg_send![button, setImagePosition: NSIMAGE_LEFT];
      let is_template = match self.icon_is_template {
        true => YES,
        false => NO,
      };
      let _: () = msg_send![nsimage, setTemplate: is_template];
    }
  }
}

/// Create a `TrayHandler` Class that handle button click event and also menu opening and closing.
///
/// We set the tray menu to tray_target instead of status bar, because setting directly to status bar
/// will overwrite the event callback of the button. When `perform_tray_click` called, it will set
/// the menu to status bar in the end. And when the menu is closed `menu_did_close` will set it to
/// nil again.
fn make_tray_class() -> *const Class {
  static mut TRAY_CLASS: *const Class = 0 as *const Class;
  static INIT: Once = Once::new();

  INIT.call_once(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("TaoTrayHandler", superclass).unwrap();
    decl.add_ivar::<id>("status_bar");
    decl.add_ivar::<id>("menu");
    decl.add_ivar::<bool>("menu_on_left_click");
    decl.add_ivar::<u16>("id");
    decl.add_method(
      sel!(click:),
      perform_tray_click as extern "C" fn(&mut Object, _, id),
    );

    let delegate = Protocol::get("NSMenuDelegate").unwrap();
    decl.add_protocol(&delegate);
    decl.add_method(
      sel!(menuDidClose:),
      menu_did_close as extern "C" fn(&mut Object, _, id),
    );

    TRAY_CLASS = decl.register();
  });

  unsafe { TRAY_CLASS }
}

/// This will fire for an NSButton callback.
extern "C" fn perform_tray_click(this: &mut Object, _: Sel, button: id) {
  unsafe {
    let id = this.get_ivar::<u16>("id");
    let app: id = msg_send![class!(NSApplication), sharedApplication];
    let current_event: id = msg_send![app, currentEvent];

    // icon position & size
    let window: id = msg_send![current_event, window];
    let frame = NSWindow::frame(window);
    let scale_factor = NSWindow::backingScaleFactor(window) as f64;
    let position: PhysicalPosition<f64> = LogicalPosition::new(
      frame.origin.x as f64,
      bottom_left_to_top_left_for_tray(frame),
    )
    .to_physical(scale_factor);

    let logical: LogicalSize<f64> = (frame.size.width as f64, frame.size.height as f64).into();
    let size: PhysicalSize<f64> = logical.to_physical(scale_factor);

    // cursor position
    let mouse_location: NSPoint = msg_send![class!(NSEvent), mouseLocation];
    // what type of click?
    let event_mask: NSEventType = msg_send![current_event, type];
    // grab the modifier flag, to make sure the ctrl + left click = right click
    let key_code: NSEventModifierFlags = msg_send![current_event, modifierFlags];

    let click_type = match event_mask {
      // left click + control key
      NSEventType::NSLeftMouseDown if key_code.contains(NSEventModifierFlags::NSControlKeyMask) => {
        Some(TrayEvent::RightClick)
      }
      NSEventType::NSLeftMouseDown => Some(TrayEvent::LeftClick),
      NSEventType::NSRightMouseDown => Some(TrayEvent::RightClick),
      _ => None,
    };

    if let Some(click_event) = click_type {
      let event = Event::TrayEvent {
        id: TrayId(*id as u16),
        bounds: Rectangle { position, size },
        position: PhysicalPosition::new(
          mouse_location.x,
          bottom_left_to_top_left_for_cursor(mouse_location),
        ),
        event: click_event,
      };

      AppState::queue_event(EventWrapper::StaticEvent(event));

      let menu = this.get_ivar::<id>("menu");
      if *menu != nil {
        let menu_on_left_click = this.get_ivar::<bool>("menu_on_left_click");
        if click_event == TrayEvent::RightClick
          || (*menu_on_left_click && click_event == TrayEvent::LeftClick)
        {
          let status_bar = this.get_ivar::<id>("status_bar");
          status_bar.setMenu_(*menu);
          let () = msg_send![button, performClick: nil];
        }
      }
    }
  }
}

// Set the menu of the status bar to nil, so it won't overwrite the button events.
extern "C" fn menu_did_close(this: &mut Object, _: Sel, _menu: id) {
  unsafe {
    let status_bar = this.get_ivar::<id>("status_bar");
    status_bar.setMenu_(nil);
  }
}
