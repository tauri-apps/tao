// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::{
  platform::macos::ActivationPolicy,
  platform_impl::platform::{
    app_state::AppState,
    ffi::{id, BOOL, YES},
  },
};

use objc2::runtime::{
  AnyClass as Class, AnyObject as Object, Bool, ClassBuilder as ClassDecl, Sel,
};
use objc2_foundation::{
  NSArray, NSError, NSString, NSUserActivity, NSUserActivityTypeBrowsingWeb, NSURL,
};
use std::{
  cell::{RefCell, RefMut},
  ffi::{CStr, CString},
  os::raw::c_void,
  sync::Mutex,
  time::Instant,
};

const AUX_DELEGATE_STATE_NAME: &str = "auxState";

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

pub struct AppDelegateClass(pub *const Class);
unsafe impl Send for AppDelegateClass {}
unsafe impl Sync for AppDelegateClass {}

lazy_static! {
  pub static ref APP_DELEGATE_CLASS: AppDelegateClass = unsafe {
    let superclass = class!(NSResponder);
    let mut decl = ClassDecl::new(
      CStr::from_bytes_with_nul(b"TaoAppDelegateParent\0").unwrap(),
      superclass,
    )
    .unwrap();

    decl.add_class_method(sel!(new), new as extern "C" fn(_, _) -> _);
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(_, _));

    decl.add_method(
      sel!(applicationDidFinishLaunching:),
      did_finish_launching as extern "C" fn(_, _, _),
    );
    decl.add_method(
      sel!(applicationWillTerminate:),
      application_will_terminate as extern "C" fn(_, _, _),
    );
    decl.add_method(
      sel!(application:openURLs:),
      application_open_urls as extern "C" fn(_, _, _, _),
    );
    decl.add_method(
      sel!(application:willContinueUserActivityWithType:),
      application_will_continue_user_activity_with_type as extern "C" fn(_, _, _, _) -> _,
    );
    decl.add_method(
      sel!(application:continueUserActivity:restorationHandler:),
      application_continue_user_activity as extern "C" fn(_, _, _, _, _) -> _,
    );
    decl.add_method(
      sel!(applicationShouldHandleReopen:hasVisibleWindows:),
      application_should_handle_reopen as extern "C" fn(_, _, _, _) -> _,
    );
    decl.add_method(
      sel!(applicationSupportsSecureRestorableState:),
      application_supports_secure_restorable_state as extern "C" fn(_, _, _) -> _,
    );
    decl.add_method(
      sel!(application:didRegisterForRemoteNotificationsWithDeviceToken:),
      did_register_for_apns as extern "C" fn(&Object, Sel, id, id),
    );
    decl.add_method(
      sel!(application:didFailToRegisterForRemoteNotificationsWithError:),
      did_fail_to_register_for_apns as extern "C" fn(&Object, _: Sel, id, id),
    );
    decl.add_ivar::<*mut c_void>(&CString::new(AUX_DELEGATE_STATE_NAME).unwrap());

    AppDelegateClass(decl.register())
  };
}

/// Safety: Assumes that Object is an instance of APP_DELEGATE_CLASS
#[allow(deprecated)] // TODO: Use define_class!
pub unsafe fn get_aux_state_mut(this: &Object) -> RefMut<'_, AuxDelegateState> {
  let ptr: *mut c_void = *this.get_ivar(AUX_DELEGATE_STATE_NAME);
  // Watch out that this needs to be the correct type
  (*(ptr as *mut RefCell<AuxDelegateState>)).borrow_mut()
}

extern "C" fn new(class: &Class, _: Sel) -> id {
  #[allow(deprecated)] // TODO: Use define_class!
  unsafe {
    let this: id = msg_send![class, alloc];
    let this: id = msg_send![this, init];
    *(*this).get_mut_ivar(AUX_DELEGATE_STATE_NAME) =
      Box::into_raw(Box::new(RefCell::new(AuxDelegateState {
        activation_policy: ActivationPolicy::Regular,
        activate_ignoring_other_apps: true,
        dock_visibility: true,
        last_dock_show: Mutex::new(None),
      }))) as *mut c_void;
    this
  }
}

extern "C" fn dealloc(this: &Object, _: Sel) {
  #[allow(deprecated)] // TODO: Use define_class!
  unsafe {
    let state_ptr: *mut c_void = *(this.get_ivar(AUX_DELEGATE_STATE_NAME));
    // As soon as the box is constructed it is immediately dropped, releasing the underlying
    // memory
    drop(Box::from_raw(state_ptr as *mut RefCell<AuxDelegateState>));
  }
}

#[cfg(feature = "push-notifications")]
fn register_push_notifications() {
  // register for push notifications. this call is inert on macOS unless the app is entitled to
  // access an APS environment. see:
  // https://developer.apple.com/documentation/usernotifications/registering-your-app-with-apns
  // and:
  // https://developer.apple.com/documentation/bundleresources/entitlements/com_apple_developer_aps-environment
  let shared_app = get_shared_application();
  unsafe {
    // registerForRemoteNotifications()
    let _: () = msg_send![shared_app, registerForRemoteNotifications];
  };
}

extern "C" fn did_finish_launching(this: &Object, _: Sel, _: id) {
  trace!("Triggered `applicationDidFinishLaunching`");

  #[cfg(feature = "push-notifications")]
  register_push_notifications();

  AppState::launched(this);
  trace!("Completed `applicationDidFinishLaunching`");
}

extern "C" fn application_will_terminate(_: &Object, _: Sel, _: id) {
  trace!("Triggered `applicationWillTerminate`");
  AppState::exit();
  trace!("Completed `applicationWillTerminate`");
}

extern "C" fn application_open_urls(_: &Object, _: Sel, _: id, urls: &NSArray<NSURL>) {
  trace!("Trigger `application:openURLs:`");

  let urls = unsafe {
    (0..urls.count())
      .flat_map(|i| url::Url::parse(&urls.objectAtIndex(i).absoluteString().unwrap().to_string()))
      .collect::<Vec<_>>()
  };
  trace!("Get `application:openURLs:` URLs: {:?}", urls);
  AppState::open_urls(urls);
  trace!("Completed `application:openURLs:`");
}

extern "C" fn application_will_continue_user_activity_with_type(
  _: &Object,
  _: Sel,
  _: id,
  user_activity_type: &NSString,
) -> Bool {
  trace!("Trigger `application:willContinueUserActivityWithType:`");
  let result = unsafe { Bool::new(user_activity_type == NSUserActivityTypeBrowsingWeb) };
  trace!("Completed `application:willContinueUserActivityWithType:`");
  result
}

extern "C" fn application_continue_user_activity(
  _: &Object,
  _: Sel,
  _: id,
  user_activity: &NSUserActivity,
  _restoration_handler: &block2::Block<dyn Fn(*mut NSError)>,
) -> Bool {
  trace!("Trigger `application:continueUserActivity:restorationHandler:`");
  let url = unsafe {
    if user_activity
      .activityType()
      .isEqualToString(NSUserActivityTypeBrowsingWeb)
    {
      match user_activity
        .webpageURL()
        .and_then(|url| url.absoluteString())
        .and_then(|s| Some(s.to_string()))
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
  return Bool::new(true);
}

extern "C" fn application_should_handle_reopen(
  _: &Object,
  _: Sel,
  _: id,
  has_visible_windows: BOOL,
) -> BOOL {
  trace!("Triggered `applicationShouldHandleReopen`");
  AppState::reopen(has_visible_windows.as_bool());
  trace!("Completed `applicationShouldHandleReopen`");
  has_visible_windows
}

extern "C" fn application_supports_secure_restorable_state(_: &Object, _: Sel, _: id) -> BOOL {
  trace!("Triggered `applicationSupportsSecureRestorableState`");
  trace!("Completed `applicationSupportsSecureRestorableState`");
  YES
}

// application(_:didRegisterForRemoteNotificationsWithDeviceToken:)
extern "C" fn did_register_for_apns(_: &Object, _: Sel, _: id, token_data: id) {
  trace!("Triggered `didRegisterForRemoteNotificationsWithDeviceToken`");
  let token_bytes = unsafe {
    if token_data.is_null() {
      trace!("Token data is null; ignoring");
      return;
    }
    let is_nsdata: bool = msg_send![token_data, isKindOfClass:class!(NSData)];
    if !is_nsdata {
      trace!("Token data is not an NSData object");
      return;
    }
    let bytes: *const u8 = msg_send![token_data, bytes];
    let length: usize = msg_send![token_data, length];
    std::slice::from_raw_parts(bytes, length).to_vec()
  };
  AppState::did_register_push_token(token_bytes);
  trace!("Completed `didRegisterForRemoteNotificationsWithDeviceToken`");
}

// application(_:didFailToRegisterForRemoteNotificationsWithError:)
extern "C" fn did_fail_to_register_for_apns(_: &Object, _: Sel, _: id, err: *mut Object) {
  trace!("Triggered `didFailToRegisterForRemoteNotificationsWithError`");

  let error_string = unsafe {
    if err.is_null() {
      "Unknown error (null error object)".to_string()
    } else {
      // Verify it's an NSError
      let is_error: bool = msg_send![err, isKindOfClass:class!(NSError)];
      if !is_error {
        trace!("Invalid error object type for push registration failure");
        return;
      }

      // Get the localizedDescription
      let description: *mut Object = msg_send![err, localizedDescription];
      if description.is_null() {
        trace!("Error had no description");
        return;
      }

      // Convert NSString to str
      let utf8: *const u8 = msg_send![description, UTF8String];
      let len: usize = msg_send![description, lengthOfBytesUsingEncoding:4];
      let bytes = std::slice::from_raw_parts(utf8, len);
      String::from_utf8_lossy(bytes).to_string()
    }
  };

  AppState::did_fail_to_register_push_token(error_string);
  trace!("Completed `didFailToRegisterForRemoteNotificationsWithError`");
}

fn get_shared_application() -> *mut Object {
  unsafe { msg_send![class!(NSApplication), sharedApplication] }
}

// application(_:didRegisterForRemoteNotificationsWithDeviceToken:)
extern "C" fn did_register_for_apns(_: &Object, _: Sel, _: id, token_data: id) {
  trace!("Triggered `didRegisterForRemoteNotificationsWithDeviceToken`");
  let token_bytes = unsafe {
    if token_data.is_null() {
      trace!("Token data is null; ignoring");
      return;
    }
    let is_nsdata: bool = msg_send![token_data, isKindOfClass:class!(NSData)];
    if !is_nsdata {
      trace!("Token data is not an NSData object");
      return;
    }
    let bytes: *const u8 = msg_send![token_data, bytes];
    let length: usize = msg_send![token_data, length];
    std::slice::from_raw_parts(bytes, length).to_vec()
  };
  AppState::did_register_push_token(token_bytes);
  trace!("Completed `didRegisterForRemoteNotificationsWithDeviceToken`");
}

// application(_:didFailToRegisterForRemoteNotificationsWithError:)
extern "C" fn did_fail_to_register_for_apns(_: &Object, _: Sel, _: id, err: *mut Object) {
  trace!("Triggered `didFailToRegisterForRemoteNotificationsWithError`");

  let error_string = unsafe {
    if err.is_null() {
      "Unknown error (null error object)".to_string()
    } else {
      // Verify it's an NSError
      let is_error: bool = msg_send![err, isKindOfClass:class!(NSError)];
      if !is_error {
        trace!("Invalid error object type for push registration failure");
        return;
      }

      // Get the localizedDescription
      let description: *mut Object = msg_send![err, localizedDescription];
      if description.is_null() {
        trace!("Error had no description");
        return;
      }

      // Convert NSString to str
      let utf8: *const u8 = msg_send![description, UTF8String];
      let len: usize = msg_send![description, lengthOfBytesUsingEncoding:4];
      let bytes = std::slice::from_raw_parts(utf8, len);
      String::from_utf8_lossy(bytes).to_string()
    }
  };

  AppState::did_fail_to_register_push_token(error_string);
  trace!("Completed `didFailToRegisterForRemoteNotificationsWithError`");
}

fn get_shared_application() -> *mut Object {
  unsafe { msg_send![class!(NSApplication), sharedApplication] }
}
