---
tao: patch
---

- `skip_taskbar` is renamed to `set_skip_taskbar`.
- `set_skip_taskbar` is now available on `Window` and is no longer behind a PlatformExt.
- `set_skip_taskbar` takes a boolean to either show or hide the window icon from the taskbar.
- Add `with_skip_taskbar` to `WindowBuilder`.