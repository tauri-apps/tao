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
  appkit::{NSButton, NSImage, NSStatusBar, NSStatusItem, NSVariableStatusItemLength, NSWindow},
  base::{id, nil, NO, YES},
  foundation::{NSData, NSInteger, NSPoint, NSRect, NSSize, NSString},
};
use objc::{
  declare::ClassDecl,
  runtime::{Class, Object, Sel},
};
use std::sync::Once;

const TRAY_ID: &str = "id";
const TRAY_STATUS_ITEM: &str = "status_item";
const TRAY_MENU: &str = "menu";
const TRAY_MENU_ON_LEFT_CLICK: &str = "menu_on_left_click";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum ClickType {
  Left,
  Right,
}

pub struct SystemTrayBuilder {
  icon: Icon,
  tray_menu: Option<Menu>,
  pub(crate) icon_is_template: bool,
  pub(crate) menu_on_left_click: bool,
  pub(crate) title: Option<String>,
}

impl SystemTrayBuilder {
  /// Creates a new SystemTray for platforms where this is appropriate.
  #[inline]
  pub fn new(icon: Icon, tray_menu: Option<Menu>) -> Self {
    Self {
      icon,
      tray_menu,
      icon_is_template: false,
      menu_on_left_click: true,
      title: None,
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
    let tray = unsafe {
      let ns_status_item =
        NSStatusBar::systemStatusBar(nil).statusItemWithLength_(NSVariableStatusItemLength);
      let _: () = msg_send![ns_status_item, retain];

      set_icon_for_ns_status_item_button(ns_status_item, &self.icon, self.icon_is_template)?;

      let button = ns_status_item.button();
      let frame: NSRect = msg_send![button, frame];
      let target: id = msg_send![make_tray_target_class(), alloc];
      let tray_target: id = msg_send![target, initWithFrame: frame];
      let _: () = msg_send![tray_target, retain];
      let _: () = msg_send![tray_target, setWantsLayer: YES];

      let system_tray = SystemTray {
        icon_is_template: self.icon_is_template,
        icon: self.icon,
        menu_on_left_click: self.menu_on_left_click,
        tray_menu: self.tray_menu,
        ns_status_bar: ns_status_item,
        title: None,
        tray_target,
      };

      // use our existing status bar
      let ns_status_item = system_tray.ns_status_bar;
      let tray_target = system_tray.tray_target;

      (*tray_target).set_ivar(TRAY_ID, tray_id.0);
      (*tray_target).set_ivar(TRAY_STATUS_ITEM, ns_status_item);
      (*tray_target).set_ivar(TRAY_MENU, nil);
      (*tray_target).set_ivar(TRAY_MENU_ON_LEFT_CLICK, system_tray.menu_on_left_click);

      let button: id = ns_status_item.button();
      let _: () = msg_send![button, addSubview: tray_target];

      // attach menu only if provided
      if let Some(menu) = system_tray.tray_menu.clone() {
        ns_status_item.setMenu_(menu.menu);

        (*tray_target).set_ivar(TRAY_MENU, menu.menu);
        let () = msg_send![menu.menu, setDelegate: ns_status_item];
      }

      // attach tool_tip if provided
      if let Some(tooltip) = tooltip {
        system_tray.set_tooltip(&tooltip);
      }

      // set up title if provided
      if let Some(title) = &system_tray.title {
        system_tray.set_title(title);
      }

      system_tray
    };

    Ok(RootSystemTray(tray))
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
  pub(crate) tray_target: id,
}

impl Drop for SystemTray {
  fn drop(&mut self) {
    self.remove();
  }
}

impl SystemTray {
  fn remove(&mut self) {
    unsafe {
      NSStatusBar::systemStatusBar(nil).removeStatusItem_(self.ns_status_bar);
      let _: () = msg_send![self.ns_status_bar, release];
    }

    unsafe {
      let _: () = msg_send![self.tray_target, removeFromSuperview];
      let _: () = msg_send![self.tray_target, release];
    }

    self.ns_status_bar = nil;
    self.tray_target = nil;
  }

  pub fn set_icon(&mut self, icon: Icon) {
    let _ = set_icon_for_ns_status_item_button(self.ns_status_bar, &icon, self.icon_is_template);
    unsafe {
      let _: () = msg_send![self.tray_target, updateDimensions];
    }
    self.icon = icon;
  }

  pub fn set_icon_as_template(&mut self, is_template: bool) {
    unsafe {
      let button = self.ns_status_bar.button();
      let nsimage: id = msg_send![button, image];
      let _: () = msg_send![nsimage, setTemplate: is_template as i8];
    }
    self.icon_is_template = is_template;
  }

  pub fn set_show_menu_on_left_click(&mut self, enable: bool) {
    unsafe {
      (*self.tray_target).set_ivar(TRAY_MENU_ON_LEFT_CLICK, enable);
    }
    self.menu_on_left_click = enable;
  }

  pub fn set_menu(&mut self, tray_menu: &Menu) {
    unsafe {
      (*self.tray_target).set_ivar(TRAY_MENU, tray_menu.menu);
      self.ns_status_bar.setMenu_(tray_menu.menu);
      let () = msg_send![tray_menu.menu, setDelegate: self.ns_status_bar];
    }
    self.tray_menu = Some(tray_menu.clone());
  }

  pub fn set_tooltip(&self, tooltip: &str) {
    unsafe {
      let tooltip = NSString::alloc(nil).init_str(tooltip);
      let _: () = msg_send![self.ns_status_bar.button(), setToolTip: tooltip];
      let _: () = msg_send![self.tray_target, updateDimensions];
    }
  }

  pub fn set_title(&self, title: &str) {
    unsafe {
      let title = NSString::alloc(nil).init_str(title);
      let _: () = msg_send![self.ns_status_bar.button(), setTitle: title];
      let _: () = msg_send![self.tray_target, updateDimensions];
    }
  }
}

fn set_icon_for_ns_status_item_button(
  ns_status_item: id,
  icon: &Icon,
  icon_is_template: bool,
) -> Result<(), crate::error::OsError> {
  // The image is to the right of the title https://developer.apple.com/documentation/appkit/nscellimageposition/nsimageleft
  const NSIMAGE_LEFT: i32 = 2;

  let png_icon = icon.inner.to_png()?;

  let (width, height) = icon.inner.get_size();

  let icon_height: f64 = 18.0;
  let icon_width: f64 = (width as f64) / (height as f64 / icon_height);

  unsafe {
    let status_item = ns_status_item;
    let button = status_item.button();

    // build our icon
    let nsdata = NSData::dataWithBytes_length_(
      nil,
      png_icon.as_ptr() as *const std::os::raw::c_void,
      png_icon.len() as u64,
    );

    let nsimage = NSImage::initWithData_(NSImage::alloc(nil), nsdata);
    let new_size = NSSize::new(icon_width, icon_height);

    button.setImage_(nsimage);
    let _: () = msg_send![nsimage, setSize: new_size];
    let _: () = msg_send![button, setImagePosition: NSIMAGE_LEFT];
    let _: () = msg_send![nsimage, setTemplate: icon_is_template as i8];
  }

  Ok(())
}

/// Create a `TrayHandler` Class that handle button click event and also menu opening and closing.
///
/// We set the tray menu to tray_target instead of status bar, because setting directly to status bar
/// will overwrite the event callback of the button. When `on_tray_click` called, it will set
/// the menu to status bar in the end. And when the menu is closed `menu_did_close` will set it to
/// nil again.
fn make_tray_target_class() -> *const Class {
  static mut TRAY_CLASS: *const Class = 0 as *const Class;
  static INIT: Once = Once::new();

  INIT.call_once(|| unsafe {
    let superclass = class!(NSView);
    let mut decl = ClassDecl::new("TaoTrayTarget", superclass).unwrap();

    decl.add_ivar::<id>(TRAY_STATUS_ITEM);
    decl.add_ivar::<id>(TRAY_MENU);
    decl.add_ivar::<bool>(TRAY_MENU_ON_LEFT_CLICK);
    decl.add_ivar::<u16>(TRAY_ID);

    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&mut Object, _));
    decl.add_method(
      sel!(mouseDown:),
      on_mouse_down as extern "C" fn(&mut Object, _, id),
    );
    decl.add_method(
      sel!(rightMouseDown:),
      on_right_mouse_down as extern "C" fn(&mut Object, _, id),
    );
    decl.add_method(
      sel!(mouseUp:),
      on_mouse_up as extern "C" fn(&mut Object, _, id),
    );
    decl.add_method(
      sel!(updateDimensions),
      update_dimensions as extern "C" fn(&mut Object, _),
    );

    extern "C" fn dealloc(this: &mut Object, _: Sel) {
      unsafe {
        this.set_ivar(TRAY_MENU, nil);
        this.set_ivar(TRAY_STATUS_ITEM, nil);

        let _: () = msg_send![super(this, class!(NSView)), dealloc];
      }
    }

    extern "C" fn on_mouse_down(this: &mut Object, _: Sel, event: id) {
      on_tray_click(this, event, ClickType::Left);
    }

    extern "C" fn on_right_mouse_down(this: &mut Object, _: Sel, event: id) {
      on_tray_click(this, event, ClickType::Right);
    }

    extern "C" fn on_mouse_up(this: &mut Object, _: Sel, _event: id) {
      unsafe {
        let ns_status_item = this.get_ivar::<id>(TRAY_STATUS_ITEM);
        let button: id = ns_status_item.button();
        let _: () = msg_send![button, highlight: NO];
      }
    }

    extern "C" fn update_dimensions(this: &mut Object, _: Sel) {
      unsafe {
        let ns_status_item = this.get_ivar::<id>(TRAY_STATUS_ITEM);
        let button: id = msg_send![*ns_status_item, button];

        let frame: NSRect = msg_send![button, frame];
        let _: () = msg_send![this, setFrame: frame];
      }
    }

    fn on_tray_click(this: &mut Object, event: id, click_type: ClickType) {
      unsafe {
        let id = this.get_ivar::<u16>(TRAY_ID);

        // icon position & size
        let window: id = msg_send![event, window];
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

        let event = Event::TrayEvent {
          id: TrayId(*id),
          bounds: Rectangle { position, size },
          position: PhysicalPosition::new(
            mouse_location.x,
            bottom_left_to_top_left_for_cursor(mouse_location),
          ),
          event: match click_type {
            ClickType::Left => TrayEvent::LeftClick,
            ClickType::Right => TrayEvent::RightClick,
          },
        };

        AppState::queue_event(EventWrapper::StaticEvent(event));

        let status_item = *this.get_ivar::<id>(TRAY_STATUS_ITEM);
        let button: id = msg_send![status_item, button];

        let menu_on_left_click = this.get_ivar::<bool>(TRAY_MENU_ON_LEFT_CLICK);
        if click_type == ClickType::Right || (*menu_on_left_click && click_type == ClickType::Left)
        {
          let menu = *this.get_ivar::<id>(TRAY_MENU);
          let has_items = if menu == nil {
            false
          } else {
            let num: NSInteger = msg_send![menu, numberOfItems];
            num > 0
          };
          if has_items {
            let _: () = msg_send![button, performClick: nil];
          } else {
            let _: () = msg_send![button, highlight: YES];
          }
        } else {
          let _: () = msg_send![button, highlight: YES];
        }
      }
    }

    TRAY_CLASS = decl.register();
  });

  unsafe { TRAY_CLASS }
}
