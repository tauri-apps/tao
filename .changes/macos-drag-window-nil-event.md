---
"tao": "patch"
---

On macOS, `Window::drag_window` no longer panics in debug builds when there is no current event (for example when the drag is requested after the mouse-down was handled); it logs a warning and returns `Ok(())` without dragging.
