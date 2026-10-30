---
"tao": patch
---

Apply non-resizable GTK window sizing before the first configure so tiling Wayland compositors do not enlarge ordinary fixed-size windows. Preserve deferred initialization for maximized windows.
