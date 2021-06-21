# Changelog

## \[0.3.0]

- Drop the event callback before exiting on macOS.
  - [52ebebbc](https://github.com/tauri-apps/tao/commit/52ebebbc5cc2913bb4b50ea7743e9f6be21e6657) Drop the event callback before exiting ([#86](https://github.com/tauri-apps/tao/pull/86)) on 2021-06-18
- Add `clipboard` api exposing `read_text` and `write_text`.
  - [cf22c902](https://github.com/tauri-apps/tao/commit/cf22c902d4c961be0f6cfba6a8c865e11073b027) feat: clipboard api ([#85](https://github.com/tauri-apps/tao/pull/85)) on 2021-06-21
- Fix LoopDestroyed to really exit the application.
  - [55e52a91](https://github.com/tauri-apps/tao/commit/55e52a9149adf37131e365fa667a97f9a24f1e4f) Fix LoopDestroy condition to really exit the app on 2021-06-01
- Implement all control flow variants
  - [16e2ac06](https://github.com/tauri-apps/tao/commit/16e2ac06c2f0c180a17fb8021005dec51a57af49) Add change file on 2021-05-19
- Add checks before focusing window
  - [1bd3b1c0](https://github.com/tauri-apps/tao/commit/1bd3b1c0fbdbf4e8a9bfade34a7e217f26114859) Add change file on 2021-05-22
- Add `is_visible` getter on `Window`
  - [c402a38b](https://github.com/tauri-apps/tao/commit/c402a38b1bbf240f4d2553c3f2b4edf86f03c270) feat: Add `is_visible` getter to `Window` ([#61](https://github.com/tauri-apps/tao/pull/61)) on 2021-05-27
- **Breaking change**: New keyboard API, including `Accelerator` and `GlobalShortcut`.

`WindowEvent::ModifiersChanged` is emitted when a new keyboard modifier is pressed. This is your responsibility to keep a local state. When the modifier is released, `ModifiersState::empty()` is emitted.

`WindowEvent::KeyboardInput` as been refactored and is exposing the event `KeyEvent`.

All menus (`ContextMenu` and `MenuBar`), now includes `Accelerator` support on Windows, macOS and Linux.

New modules available: `keyboard`, `accelerator` and `platform::global_shortcut`.

*Please refer to the docs and examples for more details.*

- [01fc43b0](https://github.com/tauri-apps/tao/commit/01fc43b05ea41463d512c0e3497971edc543ac9d) refactor: keyboards events ([#82](https://github.com/tauri-apps/tao/pull/82)) on 2021-06-21
- **Breaking change**: New menu/tray API.

System tray now expose `set_icon()` to update the tray icon after initialization. The `system_tray::SystemTrayBuilder` has been moved to the root of the package as a module and available on Windows, Linux and macOS, only when the `tray` feature is enabled. Windows expose a `remove()` function available with `SystemTrayExtWindows`.

Menu builder has been rebuilt from scratch, exposing 2 different types, `ContextMenu` and `MenuBar`.

Please refer to the docs and examples for more details.

- [7546dbd1](https://github.com/tauri-apps/tao/commit/7546dbd157b5387249337c5166849e2868c0ab7c) refactor: menu & tray ([#77](https://github.com/tauri-apps/tao/pull/77)) on 2021-06-03
- Fix match branch of run loop observer on iOS.
  - [4e9fede6](https://github.com/tauri-apps/tao/commit/4e9fede6b63d4cf60ea0a6b974a61a00bd2b47df) Add change file on 2021-05-23
- - `skip_taskbar` is renamed to `set_skip_taskbar`.
- `set_skip_taskbar` is now available on `Window` and is no longer behind a PlatformExt.
- `set_skip_taskbar` takes a boolean to either show or hide the window icon from the taskbar.
- Add `with_skip_taskbar` to `WindowBuilder`.
- [c0aac091](https://github.com/tauri-apps/tao/commit/c0aac0911e77b8b0b709d20fa9d1a3297b38e7ee) add `with_skip_taskbar` on 2021-05-29
- Add `skip_taskbar` implementation for windows
  - [83341701](https://github.com/tauri-apps/tao/commit/83341701843122f57139cf7583db345385b81d3c) feat: add `skip_taskabr` impl for windows ([#78](https://github.com/tauri-apps/tao/pull/78)) on 2021-05-29

## \[0.2.6]

- Add `is_decorated` getter on `Window`
  - [8237e2f3](https://github.com/tauri-apps/tao/commit/8237e2f3d54a012f3a89aea84112fb125863b582) add changefile on 2021-05-13
- Add `is_resizable` getter on `Window`
  - [c87f3bf9](https://github.com/tauri-apps/tao/commit/c87f3bf925df3692a44982c72cfea7484758bccc) add changefile on 2021-05-13
- Fix panic from borrowing in event loop on linux.
  - [12d7ccbc](https://github.com/tauri-apps/tao/commit/12d7ccbc4dd31ae35a45dd56542c5377d5235ecc) Fix event loop on linux on 2021-05-17
- Implement `set_focus()` and `with_focus()` for macOS, Windows and Linux.
  - [448e4c17](https://github.com/tauri-apps/tao/commit/448e4c17e2ba58257b6c23041faeae0551189536) Add change file on 2021-05-07

## \[0.2.5]

- Fix Priority import on Linux.
  - [20128896](https://github.com/tauri-apps/tao/commit/201288960e6af87a2e4ae9a75b3d53a874edbd3e) Add change file on 2021-05-17

## \[0.2.4]

- Refactor control flow implementation to wait.
  - [f5514f04](https://github.com/tauri-apps/tao/commit/f5514f04819f13493e75dfc844c7b06ec17bb071) Add change file on 2021-05-15

## \[0.2.3]

- Split feature flags (menu and tray).
  - [0035ac31](https://github.com/tauri-apps/tao/commit/0035ac31ea32fd1b04339a678e321411b779a5b6) Add changefile on 2021-05-10

## \[0.2.2]

- Add dox flag to skip link lib when building doc.
  - [565114c1](https://github.com/tauri-apps/tao/commit/565114c16a27b321e326195ad1f248ffa721c3a3) Add dox flag on 2021-05-09

## \[0.2.1]

- Update covector script to fix doc build.
  - [25f291f2](https://github.com/tauri-apps/tao/commit/25f291f2385b9505b31b8db2c74fd2aad4b7d699) Update covector script to fix doc build on 2021-05-09

## \[0.2.0]

- Update README and bump version.
  - [324eca05](https://github.com/tauri-apps/tao/commit/324eca05d3075e5610f83d1161e23ace656f586e) Update README.md on 2021-05-08
- Implement menu item varients for Linux.
  - [0637570f](https://github.com/tauri-apps/tao/commit/0637570f151f3ad6d5fcaa37dbacb84a5b53624a) Add change file on 2021-05-06
- Implement status bar on Linux.
  - [e17bce40](https://github.com/tauri-apps/tao/commit/e17bce40df35f4b8e9ff018a5e9b14f446eee899) Add change file on 2021-05-07
  - [86743720](https://github.com/tauri-apps/tao/commit/867437209f820f461239505201e7b21d8d66495c) \[skip ci] Update change file description on 2021-05-07
- Implement basic menu builder for macOS, Windows and Linux.
  - [ecd528d6](https://github.com/tauri-apps/tao/commit/ecd528d6c137c7d3ca8d5f16bb3f082b961db6d9) Add change file on 2021-05-04
  - [47640216](https://github.com/tauri-apps/tao/commit/476402167ce7209b57a180453733132b777c44f5) \[skip ci] Update changelog on 2021-05-05
- Add menu feature flag and rename status bar to system tray.
  - [06d95ad0](https://github.com/tauri-apps/tao/commit/06d95ad03c15b3da1601007ef1b81ea90310d177) Cargo fmt & clippy on 2021-05-08
- Implement basic menu builder for macOS, Windows and Linux.
  - [63868365](https://github.com/tauri-apps/tao/commit/63868365613bfd3f6c6d2155e9b3120f7fb9962e) Add change file on 2021-05-05
  - [94254074](https://github.com/tauri-apps/tao/commit/942540745e3c8365ac4d0813e9ae40dc7090a2cd) \[ci skip] Update changelog on 2021-05-06
  - [e17bce40](https://github.com/tauri-apps/tao/commit/e17bce40df35f4b8e9ff018a5e9b14f446eee899) Add change file on 2021-05-07
  - [86743720](https://github.com/tauri-apps/tao/commit/867437209f820f461239505201e7b21d8d66495c) \[skip ci] Update change file description on 2021-05-07
