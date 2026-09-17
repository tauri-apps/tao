---
"tao": patch
---

On Windows, `WindowBuilder::with_focused(false)` now only affects the first time the window is shown. Before, it made later `set_maximized` and `set_minimized` calls put the window back to its normal size.
