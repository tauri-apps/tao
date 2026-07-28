---
"tao": minor
---

**Breaking Change:**

Changed the names of the Android JNI lifecycle functions exposed by `android_binding!` as follows:

- `create` to `onFirstActivityCreate`
- `onActivityCreate` to `onCreate`
- `start` to `onStart`
- `resume` to `onResume`
- `pause` to `onPause`
- `stop` to `onStop`
- Removed `onActivitySaveInstanceState`
- `onActivityDestroy` to `onDestroy`
- `onActivityLowMemory` to `onLowMemory`

`onLowMemory` no longer takes any parameters.
