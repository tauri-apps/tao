---
"tao": "patch"
---

On Windows, fix leak of `tao::system_tray::Icon` when calling `tao::system_tray::SystemTray::set_icon` and leak of `String` when calling `tao::system_tray::SystemTray::set_tooltip`.
