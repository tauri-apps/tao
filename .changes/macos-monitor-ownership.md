---
"tao": patch
---

On macOS, mirror winit's monitor identity and `NSScreen` ownership, fixing leaked `NSScreen` and display UUID references.
`MonitorHandleExtMacOS::ns_screen()` borrows a pointer on the main thread, valid while another owner keeps the screen alive.
