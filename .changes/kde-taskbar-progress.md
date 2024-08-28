---
"tao": "patch"
---

On Linux, removed internal check for current desktop environment before applying `Window::set_progress_bar` API. This should allow `Window::set_progress_bar` to work on KDE Plasma and similar environments that support `libunity` APIs.
