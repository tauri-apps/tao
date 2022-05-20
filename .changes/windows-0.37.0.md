---
"tao": patch
---

Update the `windows` crate to the latest 0.37.0 release.

The `#[implement]` macro in `windows-implement` depends on `const` generic features which were just stabilized in `rustc` version 1.61, so this change also raises the MSRV from 1.56 to 1.61.
