---
"tao": patch
---

On Linux, fix a panic in `Window::set_ignore_cursor_events` when the window hasn't been shown yet, and keep the click-through once the window is shown.
