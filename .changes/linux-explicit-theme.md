---
tao: patch
---

On Linux, a theme set with `Window::set_theme`, `EventLoopWindowTarget::set_theme` or `WindowBuilder::with_theme` is no longer overridden when the system color scheme changes. Setting `None` follows the system again.
