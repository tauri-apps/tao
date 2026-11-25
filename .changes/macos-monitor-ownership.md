---
"tao": patch
---

On macOS, fix monitor reference leaks and use stable UUIDs for display identity.
`MonitorHandleExtMacOS::ns_screen()` borrows a pointer on the main thread, valid while another owner keeps the screen alive.
