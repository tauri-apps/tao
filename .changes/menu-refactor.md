---
"tao": minor
---

**Breaking change**: New menu/tray API.

System tray now expose `set_icon()` to update the tray icon after initialization. The `system_tray::SystemTrayBuilder` has been moved to the root of the package as a module and available on Windows, Linux and macOS, only when the `tray` feature is enabled. Windows expose a `remove()` function available with `SystemTrayExtWindows`.

Menu builder has been rebuilt from scratch, exposing 2 different types, `ContextMenu` and `MenuBar`.

Please refer to the docs and examples for more details.
