---
tao: patch
---

On Windows, do not panic when a message that drives an event-loop state transition (for example `WM_ENDSESSION`) is delivered while the event handler is already running.
