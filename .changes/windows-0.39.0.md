---
"tao": "patch"
---

Update `windows-rs` to the latest 0.39.0 release.

The `alloc` feature has been removed, which means it no longer accepts Rust `String` or `&str` parameters and implicitly converts them to `PWSTR` or `PSTR`.

For string literals, that feature was replaced with `s!()` and `w!()` macros which null terminate the string literal at compile time and convert to UTF-16 if necessary.