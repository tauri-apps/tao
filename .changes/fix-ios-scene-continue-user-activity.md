---
"tao": patch
---

Fix a panic in the iOS `scene:continueUserActivity:` handler when a universal link contains a malformed URL. The malformed URL is now logged as an error and skipped, matching the `scene:openURLContexts:` behavior.
