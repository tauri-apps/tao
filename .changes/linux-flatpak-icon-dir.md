---
"tao": patch
---

On Linux, Support tray icons in flatpak
For flatpak tray icons to work, the theme path we set with `set_icon_theme_path` should map 1:1 between the sandbox and the host machine.
The ideal directory to use for this is `$XDG_RUNTIME_DIR/app/{app_id}/` where {app_id} is the reverse DNS name of the flatpak app.
