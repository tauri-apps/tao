---
"tao": patch
---

On iOS, emit `Event::Opened` for URLs delivered in the scene connection options — custom-scheme and file URLs from the URL contexts, universal links from the user activities — so deep links open the app from a cold start. Also stop panicking on unparseable universal link URLs.
