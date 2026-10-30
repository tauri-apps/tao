---
"tao": patch
---

On Windows, fix a panic (`Option::unwrap()` on a `None` value in `keyboard.rs`) when a `WM_DEADCHAR` or `WM_SYSDEADCHAR` message arrives without a pending key-down, for example after its key-down was already dispatched on its own. It is now ignored, as `WM_CHAR` already is.
