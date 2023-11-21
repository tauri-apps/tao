---
"tao": "patch"
---

On Windows, remove `SetWindowTheme` call with `DarkMode_Explorer` theme which fixes a glitch downstream in `muda` crate when manually drawing the menu bar.
