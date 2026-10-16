---
"tao": patch
---

On iOS, emit `WindowEvent::Destroyed` when the app drops a `Window`, and stop emitting scene events for it afterwards.
