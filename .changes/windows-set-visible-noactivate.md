---
"tao": patch
---

On Windows, `Window::set_visible(true)` no longer activates a window that is not focusable (`WindowBuilder::with_focusable(false)` / `Window::set_focusable(false)`): it uses `SW_SHOWNA` for such windows instead of `SW_SHOW`, which activates even `WS_EX_NOACTIVATE` windows. `SW_SHOWNA` keeps the window's current (maximized / minimized) state and is a no-op when the window is already visible.
