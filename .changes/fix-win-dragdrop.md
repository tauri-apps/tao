---
"tao": patch
---

Fix the size of the slice passed to `DragQueryFileW` by passing `std::mem::transmute(path_buf.spare_capacity_mut())` instead of `&mut path_buf`.