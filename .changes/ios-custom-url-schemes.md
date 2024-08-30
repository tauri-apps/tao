---
"tao": patch
---

# iOS: added custom URL schemes handling in the AppDelegate class

Until now, only ["associated
domains"](https://developer.apple.com/documentation/xcode/supporting-associated-domains)
were handled, using the `application_continue` function, that implements [this
Swift method from the `UIApplicationDelegate`
class](https://developer.apple.com/documentation/uikit/uiapplicationdelegate/1623072-application).

For [custom URL
schemes](https://developer.apple.com/documentation/xcode/defining-a-custom-url-scheme-for-your-app),
I added a new `application_open_url` function that matches the signature of
[this other Swift
method](https://developer.apple.com/documentation/uikit/uiapplicationdelegate/1623112-application).

Most of the code of the pre-existing `application_continue` has been moved
into a separate `handle_deep_link` function so the new `application_open_url`
can call it as well.

I believe using the same `Event::Opened` event is appropriate in both
situations. Since the scheme is part of the URL, a listener can differentiate
between them if needed.

## Tauri:

Since we are emitting the same `Event::Opened` event, this change
works automatically with the ["Deep Linking"
plugin](https://v2.tauri.app/plugin/deep-linking/) without further
modifications.

Custom URL schemes in mobile apps are essential, for example,
when dealing with OAuth redirect URLs.
