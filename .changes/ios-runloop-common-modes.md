---
"tao": patch
---

On iOS, run the event loop observers in `kCFRunLoopCommonModes` so queued events are dispatched while a `UIScrollView` is tracking or decelerating, instead of waiting for the scroll to stop.
