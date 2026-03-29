---
"tao": patch
---

Fix `WindowBuilder::with_background_color` on Windows for undecorated windows by painting the configured color during `WM_PAINT` (in addition to `WM_ERASEBKGND`). Add `borderless_fixed` example (500×600, undecorated, non-resizable, gray background, drag to move).
