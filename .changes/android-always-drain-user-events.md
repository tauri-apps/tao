---
"tao": patch
---

Fix Android event loop hanging on first IPC call. When the `ndk_glue` event pipe and the wake fd became ready at the same time, `ALooper_pollAll` could return the fd event instead of `ALOOPER_POLL_WAKE`, so the loop never drained the user-event channel and every invoke response sat there forever. The event loop now drains pending user events on every iteration, regardless of what the poll reported.
