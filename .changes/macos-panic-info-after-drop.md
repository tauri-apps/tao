---
"tao": patch
---

On macOS, remove the event loop's run loop observers when the `EventLoop` is dropped. They outlived it, and if the main run loop kept turning afterwards (for example in an application that goes on running after `run_return` returned), they panicked with "The panic info must exist here".
