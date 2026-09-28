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
  sync::{LazyLock, Mutex},
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

pub static APP_DELEGATE_CLASS: LazyLock<AppDelegateClass> = LazyLock::new(|| unsafe {
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
  decl.add_ivar::<*mut c_void>(&CString::new(AUX_DELEGATE_STATE_NAME).unwrap());

  AppDelegateClass(decl.register())
});

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

extern "C" fn did_finish_launching(this: &Object, _: Sel, _: id) {
  trace!("Triggered `applicationDidFinishLaunching`");
  AppState::launched(this);
  trace!("Completed `applicationDidFinishLaunching`");
}

extern "C" fn application_will_terminate(_: &Object, _: Sel, _: id) {
  trace!("Triggered `applicationWillTerminate`");
  AppState::exit();
  trace!("Completed `applicationWillTerminate`");
}

fn url_from_absolute_string(absolute_string: Option<&str>) -> Option<url::Url> {
  absolute_string.and_then(|s| url::Url::parse(s).ok())
}

fn urls_from_nsurls(urls: &NSArray<NSURL>) -> Vec<url::Url> {
  (0..urls.count())
    .filter_map(|i| {
      url_from_absolute_string(
        urls
          .objectAtIndex(i)
          .absoluteString()
          .map(|s| s.to_string())
          .as_deref(),
      )
    })
    .collect()
}

extern "C" fn application_open_urls(_: &Object, _: Sel, _: id, urls: &NSArray<NSURL>) {
  trace!("Trigger `application:openURLs:`");

  let urls = urls_from_nsurls(urls);
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn nil_absolute_string_is_skipped() {
    assert_eq!(url_from_absolute_string(None), None);
  }

  #[test]
  fn empty_and_invalid_absolute_strings_are_skipped() {
    assert_eq!(url_from_absolute_string(Some("")), None);
    assert_eq!(url_from_absolute_string(Some("not a url")), None);
  }

  #[test]
  fn valid_absolute_string_is_parsed() {
    let url = url_from_absolute_string(Some("https://example.com/open")).unwrap();
    assert_eq!(url.as_str(), "https://example.com/open");
    let deeplink = url_from_absolute_string(Some("myapp://item/1")).unwrap();
    assert_eq!(deeplink.as_str(), "myapp://item/1");
  }
}
