---
"tao": patch
---

- On Linux, add `EventLoopWindowTargetExtUnix` for methods to determine if the backend is x11 or wayland.
- On Linux, add `x11` module for glutin internal use. This is basically just x11-dl, but winit secretly exports it.
- On Linux, add `WindowBuilder::with_transparent_draw` to disable the internal draw for transparent window and allows users to draw it manually.