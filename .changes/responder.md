---
"tao": patch
---

On macOS, remove `doCommandBySelector` in view since this will block the key event to responder chain.

