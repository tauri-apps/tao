---
"tao": patch
---

- On Linux, add `EventLoopWindowTargetExtUnix` for methods to determine if the backend is x11 or wayland.
- On Linux, add `x11` module for glutin internal use. This is basically just x11-dl, but winit secretly exports it.
- On Linux, add `auto_transparent` attribute so users can draw the window manually.