---
"tao": "patch"
---

Fixed undefined behavior in the Linux `raw_display_handle_rwh_06` implementations: a failed `XOpenDisplay` (null) no longer reaches `XDefaultScreen` / `NonNull::new_unchecked`, the null Wayland `wl_display` is checked, and both now return `Err(HandleError::Unavailable)`. The `rwh_05` implementations also skip the `XDefaultScreen` call when the display is null. See #1347.
