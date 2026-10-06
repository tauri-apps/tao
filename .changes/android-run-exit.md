---
"tao": patch
---

On Android, `EventLoop::run` terminates the process without running libc exit handlers or static destructors under live ART threads.
