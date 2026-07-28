---
"tao": minor
---

Changed the names of the Android JNI lifecycle functions exposed by `android_binding!` as follows:

- `onActivityCreate` to `onCreate`
- `start` to `onStart`
- `resume` to `onResume`
- `pause` to `onPause`
- `stop` to `onStop`
- `onActivitySaveInstanceState` to `onSaveInstanceState`
- `onActivityDestroy` to `onDestroy`
- `onActivityLowMemory` to `onLowMemory`
