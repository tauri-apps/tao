---
"tao": minor
---

On Linux, adds `SystemTrayBuilderExtLinux::with_temp_icon_dir` which sets a custom temp icon dir to store generated icon files.
This may be useful when the application requires icons to be stored in a specific location, such as when running in a Flatpak sandbox.
