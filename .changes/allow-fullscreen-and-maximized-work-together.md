---
"tao": patch
---

When we maximize a window or switch to fullscreen, we must consider the window status
values ​​after returning to normal window, such as size, position, and whether it is maximized.

This commit fixes the error that occurs when switching between fullscreen and maximized.

In summary, borderless-fullscreen and maximized can be toggled without considering
each other's status.
