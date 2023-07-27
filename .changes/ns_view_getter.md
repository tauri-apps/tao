---
"tao": "patch"
---

On macOS, fix `WindowExtMacOS::ns_view` returning an invalid pointer if the view was replaced by a call to `setContentView` later on.
