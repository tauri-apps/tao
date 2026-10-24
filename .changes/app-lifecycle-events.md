---
"tao": minor
---

Added the application-wide `Event::Suspended` and `Event::Resumed` events on Android and iOS, emitted when the application moves to the background and back to the foreground, unlike `WindowEvent::Suspended` and `WindowEvent::Resumed` which are emitted for each activity or scene.
