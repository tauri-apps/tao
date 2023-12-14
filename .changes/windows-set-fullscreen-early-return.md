---
"tao": "patch"
---

On Windows, fix consecutive calls to `window.set_fullscreen(Some(Fullscreen::Borderless(None)))` resulting in losing previous window state when eventually exiting fullscreen using `window.set_fullscreen(None)`.
