// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{platform::macos::ActivationPolicy, platform_impl::platform::app_state::AppState};

use objc2::{
  define_class, msg_send,
  rc::Retained,
  runtime::{AnyObject, Bool, ProtocolObject},
  DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{NSApplication, NSApplicationDelegate, NSResponder, NSUserActivityRestoring};
use objc2_foundation::{
  NSArray, NSObject, NSObjectProtocol, NSString, NSUserActivity, NSUserActivityTypeBrowsingWeb,
  NSURL,
};
use std::{
  cell::{RefCell, RefMut},
  ptr::NonNull,
  sync::Mutex,
  time::Instant,
};

pub struct AuxDelegateState {
  /// We store this value in order to be able to defer setting the activation policy until
  /// after the app has finished launching. If the activation policy is set earlier, the
  /// menubar is initially unresponsive on macOS 10.15 for example.
  pub activation_policy: ActivationPolicy,

  /// Whether the application is visible in the dock.
  pub dock_visibility: bool,
  pub last_dock_show: Mutex<Option<Instant>>,

  pub activate_ignoring_other_apps: bool,
}

/// Safety: `this` must be a Tao application delegate, accessed on the main thread.
pub unsafe fn get_aux_state_mut(this: &AppDelegate) -> RefMut<'_, AuxDelegateState> {
  this
    .downcast_ref::<AppDelegate>()
    .expect("application delegate must be an AppDelegate")
    .ivars()
    .borrow_mut()
}

impl AppDelegate {
  pub(super) fn new(mtm: MainThreadMarker) -> Retained<Self> {
    let delegate = Self::alloc(mtm).set_ivars(RefCell::new(AuxDelegateState {
      activation_policy: ActivationPolicy::Regular,
      activate_ignoring_other_apps: true,
      dock_visibility: true,
      last_dock_show: Mutex::new(None),
    }));
    unsafe { msg_send![super(delegate), init] }
  }
}

define_class!(
  #[unsafe(super(NSResponder, NSObject))]
  #[thread_kind = MainThreadOnly]
  #[name = "TaoAppDelegateParent"]
  #[ivars = RefCell<AuxDelegateState>]
  pub(crate) struct AppDelegate;

  unsafe impl NSObjectProtocol for AppDelegate {}

  unsafe impl NSApplicationDelegate for AppDelegate {
    #[unsafe(method(applicationDidFinishLaunching:))]
    fn did_finish_launching(&self, _: Option<&AnyObject>) {
      trace!("Triggered `applicationDidFinishLaunching`");
      AppState::launched(self);
      trace!("Completed `applicationDidFinishLaunching`");
    }

    #[unsafe(method(applicationWillTerminate:))]
    fn application_will_terminate(&self, _: Option<&AnyObject>) {
      trace!("Triggered `applicationWillTerminate`");
      AppState::exit();
      trace!("Completed `applicationWillTerminate`");
    }

    #[unsafe(method(application:openURLs:))]
    fn application_open_urls(&self, _: &NSApplication, urls: &NSArray<NSURL>) {
      trace!("Trigger `application:openURLs:`");

      let urls = (0..urls.count())
          .flat_map(|i| url::Url::parse(&urls.objectAtIndex(i).absoluteString().unwrap().to_string()))
          .collect::<Vec<_>>();
      trace!("Get `application:openURLs:` URLs: {urls:?}");
      AppState::open_urls(urls);
      trace!("Completed `application:openURLs:`");
    }

    #[unsafe(method(application:willContinueUserActivityWithType:))]
    fn application_will_continue_user_activity_with_type(
      &self,
      _: &NSApplication,
      user_activity_type: &NSString,
    ) -> bool {
      trace!("Trigger `application:willContinueUserActivityWithType:`");
      let result = unsafe { user_activity_type == NSUserActivityTypeBrowsingWeb };
      trace!("Completed `application:willContinueUserActivityWithType:`");
      result
    }

    #[unsafe(method(application:continueUserActivity:restorationHandler:))]
    fn application_continue_user_activity(
      &self,
      _: &NSApplication,
      user_activity: &NSUserActivity,
      _restoration_handler: &block2::DynBlock<
        dyn Fn(NonNull<NSArray<ProtocolObject<dyn NSUserActivityRestoring>>>),
      >,
    ) -> bool {
      trace!("Trigger `application:continueUserActivity:restorationHandler:`");
      let url = unsafe {
        if user_activity
          .activityType()
          .isEqualToString(NSUserActivityTypeBrowsingWeb)
        {
          match user_activity
            .webpageURL()
            .and_then(|url| url.absoluteString())
            .map(|s| s.to_string())
          {
            None => {
              error!(
                "`application:continueUserActivity:restorationHandler:`: restore webbrowsing activity but url is empty"
              );
              return Bool::new(false);
            }
            Some(url_string) => match url::Url::parse(&url_string) {
              Ok(url) => url,
              Err(err) => {
                error!(
                  "`application:continueUserActivity:restorationHandler:`: failed to parse url {err}"
                );
                return Bool::new(false);
              }
            },
          }
        } else {
          return Bool::new(false);
        }
      };

      AppState::open_urls(vec![url]);
      trace!("Completed `application:continueUserActivity:restorationHandler:`");
      true
    }

    #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
    fn application_should_handle_reopen(&self, _: &NSApplication, has_visible_windows: bool) -> bool {
      trace!("Triggered `applicationShouldHandleReopen`");
      AppState::reopen(has_visible_windows);
      trace!("Completed `applicationShouldHandleReopen`");
      has_visible_windows
    }

    #[unsafe(method(applicationSupportsSecureRestorableState:))]
    fn application_supports_secure_restorable_state(&self, _: &NSApplication) -> bool {
      trace!("Triggered `applicationSupportsSecureRestorableState`");
      trace!("Completed `applicationSupportsSecureRestorableState`");
      true
    }
  }
);
