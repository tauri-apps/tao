---
"tao": "patch"
---

Fix `Window::primary_monitor` panicking on Linux when there is no primary monitor, e.g. with Wayland.
