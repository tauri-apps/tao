---
"tao": patch
---

On macOS, make all window methods usable from other threads.

The following APIs now blocking wait for the operations to finish when called off main thread:

- `Window::set_title`
- `Window::set_outer_position`
- `Window::set_inner_size`
- `Window::set_always_on_bottom`
- `Window::set_always_on_top`
- `Window::set_ignore_cursor_events`
- `Window::set_decorations`
- `Window::set_fullscreen`
- `WindowExtMacOS::set_simple_fullscreen`

See https://github.com/tauri-apps/tao/pull/1420#discussion_r4533635133
