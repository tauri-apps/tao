---
"tao": patch
---

Skip `set_skip_taskbar` when not hiding. Reason: If Explorer is not responding, the app will hang too. Can be verified with Process Explorer by suspending explorer.exe.
