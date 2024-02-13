---
"tao": "patch"
---

On Windows, apply `ScaleFactorChanged` if new size is different than what OS reported. This fixes an issue when moving the window to another monitor and immediately maximizing it, resulting in a maximized window (i.e have `WS_MAXIMIZE` window style) but doesn't cover the monitor work area.
