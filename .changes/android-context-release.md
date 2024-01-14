---
"tao": patch
---

Fix the app crash on restart due to Android context was not released. Release the Android context when the app is destroyed to avoid assertion failure.