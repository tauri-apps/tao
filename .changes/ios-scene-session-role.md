---
"tao": patch
---

On iOS, answer `application:configurationForConnectingSceneSession:options:` for the role the system asks about instead of always returning a `UIWindowSceneSessionRoleApplication` configuration. Because this delegate method takes precedence over the app's `UIApplicationSceneManifest`, non-window scenes — CarPlay, external displays — were handed a window configuration and never connected.
