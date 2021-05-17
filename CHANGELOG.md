# Changelog

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
