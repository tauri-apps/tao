---
"tao": patch
---

Default to MOD_NOREPEAT for registering global shortcuts / hotkeys via win32 RegisterHotKey on Windows. This prevents shortcuts from repeatedly activating when the accelerator is pressed and held down, and ensures that we maintain platform-agnostic consistency.
