---
tao: minor
---

On Windows, undecorated windows with a shadow now anchor it through the DWM frame instead of emulated non-client insets: the client area equals the window rect and a border is visible on all four edges, including the top edge on Windows 10. Adds `WindowBuilder::with_undecorated_border_transparent` / `Window::set_undecorated_border_transparent` to keep the shadow without a visible border, and `Window::undecorated_frame_inset` for content that must keep the Windows 10 border lines uncovered. Breaking: the non-client frame bands of undecorated shadowed windows are gone.
