---
tao: patch
---

On macOS, fix closed windows never being freed: the `NSWindow` was retained once too often on creation (keeping its content view alive with it), and the window delegate and view did not call `[super dealloc]` nor stop observing their notifications.
