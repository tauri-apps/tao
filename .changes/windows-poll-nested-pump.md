---
"tao": patch
---

On Windows, fix a panic ("either event handler is re-entrant") when a nested message pump running inside the event handler, such as wry's `wait_with_pump` while creating a webview, delivers `PROCESS_NEW_EVENTS`. `EventLoopRunner::poll` now skips the state transition while the event handler is taken.
