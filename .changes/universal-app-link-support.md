---
"tao": patch
---

feat: MacOS: add universal applink support
by implementing `application:willContinueUserActivityWithType:` and `application:continueUserActivity:restorationHandler:`,
reusing the existing `Event::Opened { urls }` event for the user facing api.
