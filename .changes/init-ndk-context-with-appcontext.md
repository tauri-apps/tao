---
tao: minor
---

Tao now initializes [`ndk-context`](<https://docs.rs/ndk-context>) again. This happens in the first onActivityCreate call and uses an Application context instead of an Activity context.
