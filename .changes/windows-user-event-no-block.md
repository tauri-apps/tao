---
"tao": patch
---

On Windows, fix the event loop sometimes freezing while waiting for an `EventLoopProxy::send_event` event that had not arrived yet.
