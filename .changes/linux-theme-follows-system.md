---
tao: patch
---

On Linux, a window built with `WindowBuilder::with_theme` keeps that theme, and `Window::theme` of a window built without one follows the system's color scheme instead of keeping the one it had when the window was built.
