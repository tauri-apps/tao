---
"tao": minor
---

`Event::Resumed` is now only emitted when the app is actually resumed (going back to foreground) so it won't be called on app startup.
