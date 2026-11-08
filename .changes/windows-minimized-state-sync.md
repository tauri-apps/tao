---
"tao": patch
---

On Windows, keep the cached minimized state in sync when the window is minimized or restored without `WM_SYSCOMMAND` (e.g. `ShowWindow` from another process or Win+D). A stale state made later calls such as `set_resizable` and `set_decorations` silently do nothing.
