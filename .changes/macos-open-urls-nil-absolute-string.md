---
tao: patch
---

On macOS, skip `application:openURLs:` entries whose `NSURL.absoluteString` is nil instead of unwrapping, which aborted the process.
