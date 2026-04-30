---
"tao": minor
---

Moved `Suspended` and `Resumed` from `Event` to `WindowEvent` so Android and iOS lifecycle events are emitted for the specific window that was suspended or resumed.
