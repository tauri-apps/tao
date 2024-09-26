---
"tao": patch
---

On Linux Wayland, changed the event handling for maximizing to process events sequentially to avoid "Error 71(Protocol error): dispatching to Wayland display".
