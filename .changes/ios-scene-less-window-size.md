---
"tao": patch
---

On iOS, fix the initial `ScaleFactorChanged` and `Resized` events reporting a 0x0 size, and shrinking the root view to 0x0, for a window created before any scene connected (an app without a `UIApplicationSceneManifest`).
