// Copyright 2021-2025 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use objc2::{define_class, rc::Retained, MainThreadOnly};
use objc2_foundation::{
  NSBundle, NSDictionary, NSError, NSNumber, NSObject, NSObjectProtocol, NSSet, NSString,
  NSUserActivity,
};
use objc2_ui_kit::{
  UIOpenURLContext, UIScene, UISceneConnectionOptions, UISceneDelegate, UISceneSession,
};

use crate::platform_impl::platform::{app_state, ffi::id};

pub unsafe fn app_supports_multiple_scenes() -> bool {
  let application: id = msg_send![class!(UIApplication), sharedApplication];
  // this function can be called before the UIApplication is set up (class delegate registration)
  if application == std::ptr::null_mut() {
    let bundle = NSBundle::mainBundle();
    let Some(info) = bundle.infoDictionary() else {
      return false;
    };

    let key = NSString::from_str("UIApplicationSceneManifest");
    let Some(manifest) = (*info).objectForKey(&key) else {
      return false;
    };

    let manifest_dict = Retained::cast_unchecked::<NSDictionary<NSString, NSObject>>(manifest);
    let supports_key = NSString::from_str("UIApplicationSupportsMultipleScenes");
    let Some(value) = (*manifest_dict).objectForKey(&supports_key) else {
      return false;
    };

    let num = Retained::cast_unchecked::<NSNumber>(value);
    (*num).as_bool()
  } else {
    msg_send![application, supportsMultipleScenes]
  }
}

define_class!(
  #[unsafe(super(NSObject))]
  #[name = "TaoSceneDelegate"]
  #[thread_kind = MainThreadOnly]
  pub struct TaoSceneDelegate;

  unsafe impl NSObjectProtocol for TaoSceneDelegate {}

  #[allow(non_snake_case)]
  unsafe impl UISceneDelegate for TaoSceneDelegate {
    #[unsafe(method(scene:willConnectToSession:options:))]
    fn scene_willConnectToSession_options(
      &self,
      scene: &UIScene,
      _session: &UISceneSession,
      _connection_options: &UISceneConnectionOptions,
    ) {
      log::error!("connecting scene...");
      unsafe {
        app_state::connect_scene(scene);
      }
    }

    #[unsafe(method(sceneDidDisconnect:))]
    fn sceneDidDisconnect(&self, _scene: &UIScene) {
      log::error!("[lucas] disconnect");
    }

    #[unsafe(method(sceneDidBecomeActive:))]
    fn sceneDidBecomeActive(&self, _scene: &UIScene) {
      log::error!("[lucas] active");
    }

    #[unsafe(method(sceneWillResignActive:))]
    fn sceneWillResignActive(&self, _scene: &UIScene) {
      log::error!("[lucas] resign");
    }

    #[unsafe(method(sceneWillEnterForeground:))]
    fn sceneWillEnterForeground(&self, _scene: &UIScene) {
      log::error!("[lucas] foreground");
    }

    #[unsafe(method(sceneDidEnterBackground:))]
    fn sceneDidEnterBackground(&self, _scene: &UIScene) {
      log::error!("[lucas] background");
    }

    #[unsafe(method(scene:openURLContexts:))]
    fn scene_openURLContexts(&self, _scene: &UIScene, _url_contexts: &NSSet<UIOpenURLContext>) {
      log::error!("[lucas] contexts");
    }

    #[unsafe(method(stateRestorationActivityForScene:))]
    fn stateRestorationActivityForScene(
      &self,
      _scene: &UIScene,
    ) -> Option<std::ptr::NonNull<NSUserActivity>> {
      log::error!("[lucas] activity for restore");
      None
    }

    #[unsafe(method(scene:restoreInteractionStateWithUserActivity:))]
    fn scene_restoreInteractionStateWithUserActivity(
      &self,
      _scene: &UIScene,
      _state_restoration_activity: &NSUserActivity,
    ) {
      log::error!("[lucas] state with user");
    }

    #[unsafe(method(scene:willContinueUserActivityWithType:))]
    fn scene_willContinueUserActivityWithType(
      &self,
      _scene: &UIScene,
      _user_activity_type: &NSString,
    ) {
      log::error!("[lucas] continue ");
    }

    #[unsafe(method(scene:continueUserActivity:))]
    fn scene_continueUserActivity(&self, _scene: &UIScene, _user_activity: &NSUserActivity) {
      log::error!("[lucas] continue2");
    }

    #[unsafe(method(scene:didFailToContinueUserActivityWithType:error:))]
    fn scene_didFailToContinueUserActivityWithType_error(
      &self,
      _scene: &UIScene,
      _user_activity_type: &NSString,
      _error: &NSError,
    ) {
      log::error!("[lucas] fail");
    }

    #[unsafe(method(scene:didUpdateUserActivity:))]
    fn scene_didUpdateUserActivity(&self, _scene: &UIScene, _user_activity: &NSUserActivity) {
      log::error!("[lucas] update");
    }
  }
);
