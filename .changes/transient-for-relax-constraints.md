---
"tao": "patch"
---

**Breaking Change**: Changed `WindowBuilderExtUnix::with_transient_for` signature to take `&impl gtk::glib::IsA<gtk::Window>` instead of `gtk::ApplicationWindow` which covers more gtk window types and matches the underlying API call signature.
