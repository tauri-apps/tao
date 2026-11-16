---
"tao": patch
---

On macOS, a redraw that arrives through `drawRect:` while the event handler is already running on the main thread is now queued and delivered after the handler returns, instead of re-entering the handler and locking its mutex a second time on the same thread. This fixes a deadlock seen when AppKit displayed the window synchronously inside a user-event closure, for example while attaching an open panel as a sheet.
