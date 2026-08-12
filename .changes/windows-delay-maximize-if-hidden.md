---
"tao": patch
---

On Windows, delay maximizing the window if it's hidden to avoid it flashing briefly. This also changed the size and position returned to pre-maximized states if you query them with the window being hidden and called `maximize`
