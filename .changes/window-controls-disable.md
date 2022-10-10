---
"tao": patch
---

Add APIs for disabling the individual window controls on desktop platforms, `Window::set_closable`, `Window::is_closable`, `WindowBuilder::with_closable`, `Window::set_minimizable`, `Window::is_minimizable`, `WindowBuilder::with_minimizable`, `Window::set_maximizable`, `Window::is_maximizable`, `WindowBuilder::with_maximizable`. See the docs for platform-specific notes, especially regarding Linux.
