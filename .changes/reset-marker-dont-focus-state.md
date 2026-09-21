---
tao: patch
---

On Windows, fix `set_maximized` and `set_minimized` calls don't work if the window's built with `with_focused(false)`.

Note: this change will also make that if the window is created through `with_focused(false)`, subsequent `window.set_visible(true)` calls will now focus the window (the current behavior is the window only gets focus if you call `window.set_focus()` or `window.set_maximized(true)`) this is consistent with the other platforms.
