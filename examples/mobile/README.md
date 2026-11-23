# tao mobile example

An app to test tao on Android and iOS, built with [cargo-mobile2](https://github.com/tauri-apps/cargo-mobile2).
It also runs on desktop with `cargo run`.

It uses tao directly, without wry: on Android each activity shows whether a tao window is attached to it,
and the app logs the events it receives (`adb logcat -s tao-mobile` on Android).

## Setup

Install cargo-mobile2 and set up the Android SDK/NDK and Xcode as described in
[wry's mobile guide](https://github.com/tauri-apps/wry/blob/dev/MOBILE.md), then generate the projects:

```sh
cargo install --git https://github.com/tauri-apps/cargo-mobile2
cd examples/mobile
cargo mobile init
```

`gen/` is generated, except for the Android activity in `gen/android/app/src/main/java`.
Running `cargo mobile init` again might delete it, so restore it with `git checkout gen` if needed.

## Running

```sh
cargo android run # or `cargo android open` to use Android Studio
cargo apple run   # or `cargo apple open` to use Xcode
```

## What to test

Android:

- The activity that launched the app is attached to the window created on `StartCause::Init`.
- **New window** creates a window with `WindowBuilderExtAndroid::with_activity_name`,
  which starts a new activity. It must not emit `Event::ActivityRequested`.
- **Finish activity** finishes the activity while the process stays alive. Launching the app again from the
  launcher makes the system create an activity on its own, which emits `Event::ActivityRequested`;
  the example attaches a window to it with `WindowBuilderExtAndroid::with_activity_id`.
- Configuration changes that recreate the activity, e.g. toggling dark mode with `adb shell cmd uimode night yes`,
  must keep it attached to its window and not emit `Event::ActivityRequested`.

iOS:

- `Event::SceneRequested` is logged when the system requests a scene, e.g. "New window" from the app icon on iPad.
