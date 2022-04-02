---
"tao": "minor"
---

* **Breaking change** `SystemTrayBuilder::new` and `SystemTray::set_icon` now takes `Vec<u8>` instead of `PathBuf` on Linux.
* **Breaking change** `SystemTrayBuilder::new` and `SystemTray::set_icon` now takes 2 more arguments, width and height.