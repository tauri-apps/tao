---
"tao": patch
---

On macOS, fix native file dialogs hanging the event loop and
having multiple windows would prevent `run_return` from ever returning.
