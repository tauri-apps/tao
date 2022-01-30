---
"tao": minor
---

**Breaking** `SystemTray::set_icon` and `SystemTrayBuilder::new` now takes `tao::system_tray::Icon` struct instead of `Vec<u8>` on macOS and Windows and instead of `PathBuf` on Linux.