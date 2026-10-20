---
"tao": patch
---

On iOS, fix a window created with `fullscreen` rendering black on iOS 27: `setScreen:` is now only called when the window actually moves to another screen.
