---
"tao": patch
---

Fix logical `with_position` on Windows choosing the wrong monitor when monitors have different scale factors. Monitor selection now uses the closest matching origin instead of enumeration order.
