---
"tao": patch
---

Release the window state lock before updating taskbar visibility on Windows to avoid a reentrant `TaskbarCreated` deadlock.
