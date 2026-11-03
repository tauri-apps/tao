---
tao: patch
---

On Linux, `WindowEvent::ThemeChanged` is sent to each window under its own id instead of a dummy one that matches no window.
