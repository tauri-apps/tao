---
"tao": patch
---

On macOS, handle `applicationShouldTerminate:`. Dock Quit, logout, shutdown and the predefined Quit menu item called `NSApplication::terminate:` directly, so the application got no preventable event and could not ask about unsaved state. Tao now emits `WindowEvent::CloseRequested` for each Tao window and cancels that termination. When no Tao window exists, termination proceeds as before.
