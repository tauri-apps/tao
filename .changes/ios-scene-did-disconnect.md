---
"tao": patch
---

On iOS, emit `WindowEvent::Destroyed` for a scene's windows when the scene disconnects. Previously `sceneDidDisconnect:` was a no-op, so windows tied to a scene never reported their destruction when the system reclaimed the scene.
