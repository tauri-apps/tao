---
"tao": "patch"
---

Fix a deadlock on Windows when using `Window::set_visible(true)` in the `EventLoop::run` closure.