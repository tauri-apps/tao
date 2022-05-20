---
"tao": patch
---

Update the windows-rs crate to the latest 0.37.0 release. This depends on rustc version 1.61 for some `const` generic support which was just stabilized, so on Windows the MSRV is effectively 1.61 now.
