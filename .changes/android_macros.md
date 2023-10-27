---
"tao": "patch"
---

This patch contains a couple of changes to how the anroid macros:

- Changed `android_binding` macro 4th argument signature, which is a setup function that is called once when the event loop is first created, from `unsafe fn(JNIEnv, &ForeignLooper, GlobalRef)` to `unsafe fn(&str, JNIEnv, &ForeignLooper, GlobalRef)`.
- Moved `android_fn!` and `generate_package_name` macro from crate root `platform::android::prelude`
