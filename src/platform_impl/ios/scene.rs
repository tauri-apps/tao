// Copyright 2021-2025 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use objc2::{define_class, rc::Retained, MainThreadMarker, MainThreadOnly};
use objc2_foundation::{
  NSBundle, NSDictionary, NSError, NSNumber, NSObject, NSObjectProtocol, NSSet, NSString,
  NSUserActivity,
};
use objc2_ui_kit::{
  UIApplication, UIOpenURLContext, UIScene, UISceneConnectionOptions, UISceneDelegate,
  UISceneSession, UISceneWindowingControlStyle, UIWindowScene, UIWindowSceneDelegate,
};

use crate::{
  event::{Event, WindowEvent},
  platform_impl::platform::{app_state, event_loop::EventWrapper},
  window::WindowId as RootWindowId,
};

pub(crate) fn emit_opened_from_url_contexts(url_contexts: &NSSet<UIOpenURLContext>) {
  let url_strings: Vec<String> = url_contexts
    .iter()
    .filter_map(|ctx| ctx.URL().absoluteString().map(|s| s.to_string()))
    .collect();

  let urls = parse_url_strings(&url_strings);
  if !urls.is_empty() {
    unsafe {
      app_state::handle_nonuser_event(EventWrapper::StaticEvent(Event::Opened { urls }));
    }
  }
}

// true when the system allows the app to display multiple scenes and multiple_scenes_enabled() returns true
// https://developer.apple.com/documentation/uikit/uiapplication/supportsmultiplescenes?language=objc
pub unsafe fn app_supports_multiple_scenes() -> bool {
  let mtm = MainThreadMarker::new().unwrap();
  let application = UIApplication::sharedApplication(mtm);
  application.supportsMultipleScenes()
}

// check whether the app's Info.plist enabled multiple scenes
pub unsafe fn multiple_scenes_enabled() -> bool {
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
      connection_options: &UISceneConnectionOptions,
    ) {
      unsafe {
        app_state::connect_scene(scene, connection_options);
      }
    }

    #[unsafe(method(sceneDidDisconnect:))]
    fn sceneDidDisconnect(&self, _scene: &UIScene) {}

    #[unsafe(method(sceneDidBecomeActive:))]
    fn sceneDidBecomeActive(&self, scene: &UIScene) {
      unsafe {
        if let Some(window_scene) = scene.downcast_ref::<UIWindowScene>() {
          for window in window_scene.windows() {
            app_state::handle_nonuser_event(EventWrapper::StaticEvent(Event::WindowEvent {
              window_id: RootWindowId(window.into()),
              event: WindowEvent::Focused(true),
            }));
          }
        }
      }
    }

    #[unsafe(method(sceneWillResignActive:))]
    fn sceneWillResignActive(&self, scene: &UIScene) {
      unsafe {
        if let Some(window_scene) = scene.downcast_ref::<UIWindowScene>() {
          for window in window_scene.windows() {
            app_state::handle_nonuser_event(EventWrapper::StaticEvent(Event::WindowEvent {
              window_id: RootWindowId(window.into()),
              event: WindowEvent::Focused(false),
            }));
          }
        }
      }
    }

    #[unsafe(method(sceneWillEnterForeground:))]
    fn sceneWillEnterForeground(&self, _scene: &UIScene) {}

    #[unsafe(method(sceneDidEnterBackground:))]
    fn sceneDidEnterBackground(&self, _scene: &UIScene) {}

    #[unsafe(method(scene:openURLContexts:))]
    fn scene_openURLContexts(&self, _scene: &UIScene, url_contexts: &NSSet<UIOpenURLContext>) {
      emit_opened_from_url_contexts(url_contexts);
    }

    #[unsafe(method(stateRestorationActivityForScene:))]
    fn stateRestorationActivityForScene(
      &self,
      _scene: &UIScene,
    ) -> Option<std::ptr::NonNull<NSUserActivity>> {
      None
    }

    #[unsafe(method(scene:restoreInteractionStateWithUserActivity:))]
    fn scene_restoreInteractionStateWithUserActivity(
      &self,
      _scene: &UIScene,
      _state_restoration_activity: &NSUserActivity,
    ) {
    }

    #[unsafe(method(scene:willContinueUserActivityWithType:))]
    fn scene_willContinueUserActivityWithType(
      &self,
      _scene: &UIScene,
      _user_activity_type: &NSString,
    ) {
    }

    #[unsafe(method(scene:continueUserActivity:))]
    fn scene_continueUserActivity(&self, _scene: &UIScene, user_activity: &NSUserActivity) {
      unsafe {
        // universal app links
        if let Some(url) = user_activity
          .webpageURL()
          .and_then(|url| url.absoluteString())
        {
          let urls = parse_url_strings(&[url.to_string()]);
          if !urls.is_empty() {
            app_state::handle_nonuser_event(EventWrapper::StaticEvent(Event::Opened { urls }));
          }
        }
      }
    }

    #[unsafe(method(scene:didFailToContinueUserActivityWithType:error:))]
    fn scene_didFailToContinueUserActivityWithType_error(
      &self,
      _scene: &UIScene,
      _user_activity_type: &NSString,
      _error: &NSError,
    ) {
    }

    #[unsafe(method(scene:didUpdateUserActivity:))]
    fn scene_didUpdateUserActivity(&self, _scene: &UIScene, _user_activity: &NSUserActivity) {}
  }

  #[allow(non_snake_case)]
  unsafe impl UIWindowSceneDelegate for TaoSceneDelegate {
    #[unsafe(method(preferredWindowingControlStyleForScene:))]
    fn preferredWindowingControlStyleForScene(
      &self,
      _window_scene: &UIWindowScene,
    ) -> Option<std::ptr::NonNull<UISceneWindowingControlStyle>> {
      std::ptr::NonNull::new(Retained::autorelease_ptr(
        UISceneWindowingControlStyle::minimalStyle(),
      ))
    }
  }
);

fn parse_url_strings(url_strings: &[String]) -> Vec<url::Url> {
  url_strings
    .iter()
    .filter_map(|s| {
      s.parse()
        .map_err(|e| {
          log::error!("failed to parse URL {s}: {e}");
          e
        })
        .ok()
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_url_strings() {
    let input = vec![
      "https://example.com".to_string(),
      "invalid-url".to_string(),
      "https://another.com/path".to_string(),
    ];

    let urls = parse_url_strings(&input);
    assert_eq!(urls.len(), 2);
    // url::Url normalizes an empty path to "/"
    assert_eq!(urls[0].as_str(), "https://example.com/");
    assert_eq!(urls[1].as_str(), "https://another.com/path");
  }

  #[test]
  fn test_parse_url_strings_empty_input() {
    assert!(parse_url_strings(&[]).is_empty());
  }

  #[test]
  fn test_parse_url_strings_all_invalid() {
    let input = vec![
      String::new(),
      "not a url".to_string(),
      // parses up to the host, which is empty
      "https://".to_string(),
    ];
    assert!(parse_url_strings(&input).is_empty());
  }

  #[test]
  fn test_parse_url_strings_deep_link_round_trip() {
    // the payloads scene:openURLContexts: actually delivers: custom URL
    // schemes, universal links with query and fragment, and file URLs
    // from the share sheet's "Open in..."
    let cases = [
      "tauri://callback?token=abc",
      "myapp:main",
      "https://example.com/auth?code=1&state=2#frag",
      "file:///private/var/mobile/Containers/doc%20name.pdf",
    ];

    for case in cases {
      let urls = parse_url_strings(&[case.to_string()]);
      assert_eq!(urls.len(), 1, "expected {case} to parse");
      assert_eq!(urls[0].as_str(), case);
    }

    let urls = parse_url_strings(&["tauri://callback?token=abc".to_string()]);
    assert_eq!(urls[0].scheme(), "tauri");
    assert_eq!(urls[0].host_str(), Some("callback"));
    assert_eq!(urls[0].query(), Some("token=abc"));
  }

  #[test]
  fn test_parse_url_strings_normalization() {
    // consumers receive URLs normalized per the WHATWG URL spec
    let cases = [
      ("HTTPS://EXAMPLE.COM/Path", "https://example.com/Path"),
      ("https://example.com/a b", "https://example.com/a%20b"),
      ("https://example.com:443/page", "https://example.com/page"),
    ];

    for (input, expected) in cases {
      let urls = parse_url_strings(&[input.to_string()]);
      assert_eq!(urls.len(), 1, "expected {input} to parse");
      assert_eq!(urls[0].as_str(), expected);
    }
  }
}
