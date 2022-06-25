---
"tao": patch
---

On Linux, store tray icons in `$XDG_RUNTIME_DIR`.
This is preferred over `/tmp`, because this directory (typically `/run/user/{uid}`)
is only readable for the current user. While `/tmp` is shared with all users.
