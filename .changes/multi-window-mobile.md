---
"tao": minor
---

Added multi-window support for iOS and Android.

Leverages [scenes](https://developer.apple.com/documentation/uikit/scenes) on iOS and [Activity embedding](https://developer.android.com/develop/ui/views/layout/activity-embedding) on Android.

iOS:

- Added Event::SceneRequested (on iPad the user can request a new window to be open - e.g. by long pressing the app icon and selecting "New window")
- Request new scene to be created on Window::new (if needed, main scene is detected automatically) and assign the window instance later when it gets connected

Android:

- Create new activity on Window::new (if needed, main activity is detected automatically)
- Added builder methods to determine the activity to be created
- System determines what to do with the activity (new stack, next to another one.. based on the embedding rules)
