---
"tao": patch
---

On macOS and iOS, declare the Objective-C method implementations and CFRunLoop callbacks `extern "C-unwind"`, so an Objective-C exception raised beneath one (for example inside `[super sendEvent:]`) unwinds to the system's handler instead of aborting the process with "panic in a function that cannot unwind". Apps built with `panic = "abort"` still abort.
