---
"tao": minor
---

On macOS, expose the CrApp protocol (`isHandlingSendEvent` /
`setHandlingSendEvent:`) on the `TaoApp` `NSApplication` subclass.
This lets a Chromium / CEF embedder coordinate its event-pump work
with the host's `sendEvent:` dispatch without having to subclass or
swizzle `NSApp` itself. Purely additive — existing tao users see no
behavior change because nothing else calls these selectors.
