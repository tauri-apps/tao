---
"tao": patch
---

Fix a Linux session crash where every touch event near the border of an undecorated window re-issued `begin_resize_drag_for_device`; a resize drag now only starts on touch begin.
