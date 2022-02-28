---
"tao": "minor"
---

**Breaking change** `SystemTrayBuilder::new` and `SystemTray::set_icon` now takes `Icon` struct instead of `Vec<u8>` on Windows and macOS and instead of `PathBuf` on Linux.
