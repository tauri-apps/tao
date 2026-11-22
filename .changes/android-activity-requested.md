---
tao: minor
---

On Android, added `Event::ActivityRequested`, emitted when the system creates an activity that is not attached to a window, for instance when the app is launched again while a foreground service kept its process alive. Added `WindowBuilderExtAndroid::with_activity_id` to attach a window to that activity.
