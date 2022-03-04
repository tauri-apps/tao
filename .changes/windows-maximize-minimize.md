---
"tao": "patch"
---

On Windows, apply maximize state before minimize. Fixes `Window::set_minimized` not working when the window is maximized.