---
"tao": patch
---

On macOS, screen-coordinate conversions mixed Cocoa points with `CGDisplay` pixels: the y-flip helpers used `pixels_high()` where the frame values are points, `MonitorHandle::size` fed pixel extents through a logical conversion, and `cursor_position` converted with the primary monitor's scale regardless of which monitor the cursor was on. Window and monitor positions, sizes, and the cursor position are now computed in points and converted with the correct monitor's scale factor, fixing wrong geometry on any system whose primary display scale is not 1.0. `set_outer_position` now converts physical targets with the scale of the monitor the position lands on, so moving a window to `outer_position()` no longer relocates it across monitors, and `MonitorHandle::scale_factor` derives the backing scale from CoreGraphics when the `NSScreen` lookup fails instead of silently reporting 1.0.
