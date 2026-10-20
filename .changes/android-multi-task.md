---
"tao": minor
---

On Android, each window is now an activity in a task of its own (`FLAG_ACTIVITY_NEW_DOCUMENT | FLAG_ACTIVITY_MULTIPLE_TASK`), which the system shows as a separate window (Recents entry, split-screen pane, desktop window). Windows were previously stacked on the task of the activity that opened them.

- Without `WindowBuilderExtAndroid::with_activity_name`, a window uses the activity the system started (the launcher activity on app start) when no window uses it yet, and otherwise a new instance of the launcher activity.
- Dropping a `Window` finishes its activity and removes its task.
- A window no longer takes an activity that already hosts another window.
- The new `WindowExtAndroid::create_embedded_activity` starts an activity in a window's task to host another webview, which activity embedding rules can show next to the window's own. The returned `EmbeddedActivity` finishes it when dropped.
- `AndroidContext::create_activity` now takes an optional activity name and an `ActivityLaunch`.

Requires a wry version whose `WryActivity` provides `startActivity(Class, boolean)` and `closeActivity(boolean)`.
