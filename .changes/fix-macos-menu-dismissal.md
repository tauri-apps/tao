---
"tao": patch
---

On macOS, register the event-loop control-flow observers, the `EventLoopWaker` timer, and the `EventLoopProxy` source in `kCFRunLoopDefaultMode` + `NSModalPanelRunLoopMode` instead of `kCFRunLoopCommonModes`. This keeps them out of `NSEventTrackingRunLoopMode`, so opening a native menu (e.g. an `NSStatusItem`/tray-icon menu, or any `NSMenu`) no longer causes tao to pump the event loop during the menu's nested tracking loop and dismiss it.
