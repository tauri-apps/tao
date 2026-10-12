---
"tao": patch
---

On iOS, emit `Event::Opened` for the URLs and universal links found in the `UISceneConnectionOptions` of a connecting scene. When the app is launched by a URL, UIKit hands it over there and does not call `scene:openURLContexts:`, so the URL was lost.
