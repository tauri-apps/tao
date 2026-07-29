---
"tao": patch
---

On Windows, use `Arc` for the internal event loop runner handle, fixing use-after-free crashes when the event loop window target is cloned and dropped across threads (tauri-apps/tauri#15408).
