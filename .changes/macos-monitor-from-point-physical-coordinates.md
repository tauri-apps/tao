---
"tao": patch
---

The `x` and `y` values passed to `Monitor::from_point` are now treated as physical and therefore converted to logical values on macOS.
This makes the behavior consistent with the implementation for Windows.
