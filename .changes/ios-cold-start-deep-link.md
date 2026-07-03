---
"tao": patch
---

On iOS, emit `Event::Opened` for URLs delivered in the scene connection options, so deep links open the app from a cold start. Also stop panicking on unparseable universal link URLs.
