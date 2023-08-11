---
"tao": "minor"
---

This release contains a number of **breaking changes** that aimed at removing menus, system-tray and global-shortcuts features which have been moved to different crates, [`muda`](https://github.com/tauri-apps/muda/), [`tray-icon`](https://github.com/tauri-apps/tray-icon/) and [`global-hotkey`](https://github.com/tauri-apps/global-hotkey) and here is a summary of the changes:

- Removed `tray` crago feature flag.
- Removed `accelerator`, `menu`, `system_tray` and `global_shortcut` modules and all associated types.
- Removed `Event::MenuEvent`, `Event::TrayEvent`, `Event::GlobalShortcutEvent`, `TrayEvent` and `Rectangle` types.
- Added `EventLoopBuilder` type.
- Removed `EventLoop::with_user_event`, instead use `EventLoopBuilder::<T>::with_user_event().build()`.
- Removed `EventLoopExtWindows`, `EventLoopExtMacOS` and `EventLoopExtUnix`, instead use `EventLoopBuilderExtWindows`, `EventLoopBuilderExtMacOS` and `EventLoopBuilderExtUnix`.
- Changed `WindowExtWindows::hinstance`, `WindowExtWindows::hwnd` and `MonitorHandleExtWindow::hmonitor` to return `isize` instead of `*const c_void`
