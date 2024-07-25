---
"tao": minor
---

**Breaking change**: Upgrade `ndk` crate to `0.9` and `ndk-sys` crate to `0.6`.  Types from the `ndk` crate are used in public API surface.
**Breaking change**: Change `NativeKeyCode::Android(u32)` type to use `i32`, which is the native type used by all Android API.
**Breaking change**: The `setup` function passed to `android_binding!()` must now take a `&ThreadLooper` instead of `&ForeignLooper`, matching the `wry` change in https://github.com/tauri-apps/wry/pull/1296.
