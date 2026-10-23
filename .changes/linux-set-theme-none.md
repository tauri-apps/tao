---
"tao": patch
---

On Linux and BSD, `Window::set_theme(None)` and `EventLoopWindowTarget::set_theme(None)` now use the XDG desktop portal's color scheme (with the `dbus` feature) instead of turning off GTK's dark preference and emitting `ThemeChanged(Light)`.
