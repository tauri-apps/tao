---
tao: patch
---

Fix `EventLoopProxy::send_event` sometimes doesn't deliver the event until the next `EventLoopProxy::send_event` call
