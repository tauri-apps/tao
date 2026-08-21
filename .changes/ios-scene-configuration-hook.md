---
"tao": patch
---

On iOS, always implement `application:configurationForConnectingSceneSession:options:` on the application delegate, using the role of the connecting session, so the tao scene delegate is installed whenever the system runs the app on the scene lifecycle - which iOS 26 and above does even for apps whose Info.plist does not declare a `UIApplicationSceneManifest`. Windows are now attached to a scene whenever one is connected instead of only when the Info.plist enables multiple scenes, and windows created before the first scene connects are adopted by it, so they are still displayed.
