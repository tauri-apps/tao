// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use std::collections::VecDeque;

use objc2::{define_class, MainThreadOnly};
use objc2_app_kit::{self as appkit, NSApplication, NSEvent, NSEventType, NSResponder};
use objc2_foundation::NSObject;

use super::{app_state::AppState, event::EventWrapper, util, DEVICE_ID};
use crate::event::{DeviceEvent, ElementState, Event};

define_class!(
  #[unsafe(super(NSApplication, NSResponder, NSObject))]
  #[thread_kind = MainThreadOnly]
  #[name = "TaoApp"]
  pub(super) struct TaoApp;

  impl TaoApp {
    // Normally, holding Cmd + any key never sends us a `keyUp` event for that key.
    // Overriding `sendEvent:` like this fixes that. (https://stackoverflow.com/a/15294196)
    // Fun fact: Firefox still has this bug! (https://bugzilla.mozilla.org/show_bug.cgi?id=1299553)
    #[unsafe(method(sendEvent:))]
    fn send_event(&self, event: &NSEvent) {
      unsafe {
        // For posterity, there are some undocumented event types
        // (https://github.com/servo/cocoa-rs/issues/155)
        // but that doesn't really matter here.
        let event_type = event.r#type();
        let modifier_flags = event.modifierFlags();
        if event_type == appkit::NSKeyUp
          && util::has_flag(modifier_flags, appkit::NSEventModifierFlags::Command)
        {
          if let Some(key_window) = self.keyWindow() {
            key_window.sendEvent(event);
          } else {
            log::debug!("skip sending CMD keyEvent - app has no keyWindow");
          }
        } else {
          maybe_dispatch_device_event(event);
          let _: () = msg_send![super(self), sendEvent: event];
        }
      }
    }
  }
);

unsafe fn maybe_dispatch_device_event(event: &NSEvent) {
  let event_type = event.r#type();
  match event_type {
    NSEventType::MouseMoved
    | NSEventType::LeftMouseDragged
    | NSEventType::OtherMouseDragged
    | NSEventType::RightMouseDragged => {
      let mut events = VecDeque::with_capacity(3);

      let delta_x = event.deltaX() as f64;
      let delta_y = event.deltaY() as f64;

      if delta_x != 0.0 {
        events.push_back(EventWrapper::StaticEvent(Event::DeviceEvent {
          device_id: DEVICE_ID,
          event: DeviceEvent::Motion {
            axis: 0,
            value: delta_x,
          },
        }));
      }

      if delta_y != 0.0 {
        events.push_back(EventWrapper::StaticEvent(Event::DeviceEvent {
          device_id: DEVICE_ID,
          event: DeviceEvent::Motion {
            axis: 1,
            value: delta_y,
          },
        }));
      }

      if delta_x != 0.0 || delta_y != 0.0 {
        events.push_back(EventWrapper::StaticEvent(Event::DeviceEvent {
          device_id: DEVICE_ID,
          event: DeviceEvent::MouseMotion {
            delta: (delta_x, delta_y),
          },
        }));
      }

      AppState::queue_events(events);
    }
    NSEventType::LeftMouseDown | NSEventType::RightMouseDown | NSEventType::OtherMouseDown => {
      let mut events = VecDeque::with_capacity(1);

      events.push_back(EventWrapper::StaticEvent(Event::DeviceEvent {
        device_id: DEVICE_ID,
        event: DeviceEvent::Button {
          button: event.buttonNumber() as u32,
          state: ElementState::Pressed,
        },
      }));

      AppState::queue_events(events);
    }
    NSEventType::LeftMouseUp | NSEventType::RightMouseUp | NSEventType::OtherMouseUp => {
      let mut events = VecDeque::with_capacity(1);

      events.push_back(EventWrapper::StaticEvent(Event::DeviceEvent {
        device_id: DEVICE_ID,
        event: DeviceEvent::Button {
          button: event.buttonNumber() as u32,
          state: ElementState::Released,
        },
      }));

      AppState::queue_events(events);
    }
    _ => (),
  }
}
