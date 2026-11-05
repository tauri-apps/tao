---
"tao": patch
---

On Windows, fix the event loop thread blocking while delivering an `EventLoopProxy` user event when the sending thread is preempted between posting the wake-up message and queuing the event.
