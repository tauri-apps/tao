---
"tao": minor
---

* Add DeviceEventFilter on Windows.
* **Breaking**: On Windows, device events are now ignored for unfocused windows by default, use `EventLoopWindowTarget::set_device_event_filter` to set the filter level.

