---
"tao": patch
---

On macOS, report mouse buttons past the third (such as the back and forward side buttons) as `MouseButton::Other` with their AppKit button number, instead of as `MouseButton::Middle`.
