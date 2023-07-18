---
"tao": patch
---

On Android, use a lockfree queue (crossbeam channel) to prevent deadlocks inside send_event.
