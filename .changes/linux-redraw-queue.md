---
"tao": patch
---

On Linux, process pending redraw requests while waiting and coalesce duplicate requests per window, preventing the redraw queue from growing during continuous GTK drawing without other events.
