---
"tao": patch
---

On macOS, fix the event loop spinning at 100% CPU for up to several minutes after the main thread was blocked for a long time (for example behind a modal keychain prompt while the Mac was idle). The `EventLoopWaker` timer's callback now sets its own next fire date, so CoreFoundation no longer steps a late 0.1µs repeating timer forward one interval at a time.
