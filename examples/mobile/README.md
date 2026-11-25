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

`gen/` is generated, except for the Android sources in `gen/android/app/src/{main/java,debug,release}`.
Running `cargo mobile init` again might delete it, so restore it with `git checkout gen` if needed.

## Running

```sh
cargo android run # or `cargo android open` to use Android Studio
cargo apple run   # or `cargo apple open` to use Xcode
```

## End-to-end tests

`e2e/android.sh` builds the example, installs it on the connected device or emulator and drives it with adb,
checking the `[e2e]` lines the example logs and the "tao window attached" label of the foreground activity:

```sh
e2e/android.sh                        # build, install and run every scenario
e2e/android.sh --no-build rotation    # run some scenarios with the installed APK
```

| Scenario                             | Expected                                                                      |
| ------------------------------------ | ----------------------------------------------------------------------------- |
| `cold_launch`                        | the launching activity is attached to the first window, no `ActivityRequested` |
| `rotation`                           | handled by the activity, not recreated, no `ActivityRequested`                 |
| `recreation`                         | dark mode recreates the activity with the same ID, it stays attached           |
| `new_window`                         | `Window::new` with an activity name creates an activity, no `ActivityRequested` |
| `back_navigation`                    | `ActivityRequested` on relaunch only if back destroyed the root activity        |
| `finish_and_relaunch`                | process alive without activities, relaunch emits `ActivityRequested`            |
| `swipe_away_with_background_task`    | a foreground service keeps the process alive, relaunch emits `ActivityRequested` |
| `swipe_away_without_background_task` | the process dies with its task, relaunch is a cold launch                      |
| `process_death`                      | the activity is restored from its saved state in a new process, no `ActivityRequested` |

Swiping the app away from the recents screen is simulated with the gesture navigation of an emulator,
so it might need adjustments on other devices.

The **Start background task** button starts the foreground service, declared in
`gen/android/app/src/{debug,release}/AndroidManifest.xml` since cargo-mobile2 generates the main manifest.

## iOS

`Event::SceneRequested` is logged when the system requests a scene, e.g. "New window" from the app icon on iPad.
There are no automated iOS tests yet.
