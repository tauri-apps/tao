---
"tao": patch
---

On Windows, fix the monitor chosen for a logical `with_position` depending on monitor enumeration order when monitors have different scale factors. The monitor whose scale factor maps the position closest to its own origin is now picked.
