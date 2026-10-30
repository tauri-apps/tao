---
"tao": minor
---

On Linux, add a full-window `gtk::Fixed` overlay layer to each window (alongside the default vertical `gtk::Box`), exposed via `WindowExtUnix::content_fixed()`. A runtime can build positioned child webviews into this `gtk::Fixed` so they overlay the window and honor their bounds, instead of stacking beside the default box (GTK divides a vertical box among its children, which breaks multi-webview positioning — tauri-apps/tauri#10420). The default box stays the overlay's main child, so single-webview windows are unchanged.
