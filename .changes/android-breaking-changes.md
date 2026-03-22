---
"tao": minor
---

**Breaking change:** The Android activity should now reference and call the following external functions:

```
private external fun onActivityCreate(activity)
private external fun start()
private external fun resume()
private external fun pause()
private external fun stop()
private external fun onActivitySaveInstanceState()
private external fun onActivityDestroy(activity)
private external fun onActivityLowMemory()
```
