---
"tao": patch
---

On iOS, fixed a startup panic in debug builds on iOS versions older than 26, caused by registering the iOS 26-only `preferredWindowingControlStyleForScene:` method inside the `UIWindowSceneDelegate` protocol block.
