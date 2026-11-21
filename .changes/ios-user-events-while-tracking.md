---
"tao": patch
---

On iOS, deliver user events sent through an `EventLoopProxy` while UIKit runs the main loop in `UITrackingRunLoopMode` (a scroll view being dragged or decelerating), instead of holding them until the scroll ends. Each proxy's run-loop source also gets its own context `info`, since the run loop treats sources with equal contexts as one source and only ever scheduled the first proxy's.
