---
"tao": patch
---

On iOS with the scene lifecycle, emit `Event::Opened` for the URLs and universal links a scene is launched with, read from its connection options. Before, a URL that cold-started the app was dropped.
