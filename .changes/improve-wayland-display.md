---
"tao": patch
---

Changed the event handling for maximizing to process events sequentially to avoid "Error 71(Protocol error): dispatching to Wayland display".

Added buttons for maximize and minimize in the Wayland title bar.

Fixed an issue where the window was not resizing when dragging the window borders.

Fixed an issue where the window was not moving when dragging the header bar area.
