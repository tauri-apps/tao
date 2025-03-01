---
"tao": patch
---

Fixed application crash during startup when certain window buttons are disabled in the configuration on macOS. The crash occurred because the code unconditionally called `unwrap()` on an Option value returned by `standardWindowButton`, which becomes `None` when window buttons are disabled.
