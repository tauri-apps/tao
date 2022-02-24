<img src=".github/splash.png" alt="TAO - Window Creation Library" />

[![](https://img.shields.io/crates/v/tao?style=flat-square)](https://crates.io/crates/tao) [![](https://img.shields.io/docsrs/tao?style=flat-square)](https://docs.rs/tao/) ![](https://img.shields.io/crates/l/tao?style=flat-square)

[![Chat Server](https://img.shields.io/badge/chat-on%20discord-7289da.svg)](https://discord.gg/SpmNs4S)
[![devto](https://img.shields.io/badge/blog-dev.to-black.svg)](https://dev.to/tauri)
[![devto](https://img.shields.io/badge/documentation-tauri.studio-purple.svg)](https://tauri.studio/docs/getting-started/intro)
[![https://good-labs.github.io/greater-good-affirmation/assets/images/badge.svg](https://good-labs.github.io/greater-good-affirmation/assets/images/badge.svg)](https://good-labs.github.io/greater-good-affirmation)
[![support](https://img.shields.io/badge/sponsor-open%20collective-blue.svg)](https://opencollective.com/tauri)


Cross-platform application window creation library in Rust that supports all major platforms like 
Windows, macOS, Linux, iOS and Android. Built for you, maintained for Tauri.

### Cargo Features

Tao provides the following features, which can be enabled in your `Cargo.toml` file:
* `serde`: Enables serialization/deserialization of certain types with [Serde](https://crates.io/crates/serde).
* `tray`: Enables system tray and more menu item variants on **Linux**. This flag is enabled by default.
  You can still create those types if you disable it. They just don't create the actual objects. We set this flag because some implementations require more installed packages. Disable this if you don't want to install `libappindicator` package.
* `ayatana`: Enable this if you wish to use more update `libayatana-appindicator` since `libappindicator` is no longer
  maintained.

## Platform-specific notes

### Android

This library makes use of the [ndk-rs](https://github.com/rust-windowing/android-ndk-rs) crates, refer to that repo for more documentation.

Running on an Android device needs a dynamic system library, add this to Cargo.toml:
```toml
[[example]]
name = "request_redraw_threaded"
crate-type = ["cdylib"]
```

And add this to the example file to add the native activity glue:

```rust
#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
fn main() {
    ...
}
```

And run the application with `cargo apk run --example request_redraw_threaded`

### Linux

Gtk and its related libraries are used to build the support of Linux. Be sure to install following packages before building:

#### Arch Linux / Manjaro:

```bash
sudo pacman -S gtk3 libappindicator-gtk3
```

#### Debian / Ubuntu:

```bash
sudo apt install libgtk-3-dev libappindicator3-dev
```

#### MacOS

To ensure compatibility with older MacOS systems, tao links to
CGDisplayCreateUUIDFromDisplayID through the CoreGraphics framework.
However, under certain setups this function is only available to be linked
through the newer ColorSync framework. So, tao provides the
`TAO_LINK_COLORSYNC` environment variable which can be set to `1` or `true`
while compiling to enable linking via ColorSync.

### Acknowledgement

This is a fork of [winit](https://crates.io/crates/winit) which replaces Linux's port to Gtk.
We need it not only because webkit2gtk, but also a lot Desktop Environment features like menu bar, system tray, global shortcuts etc.
In the future, we want to make these features more modular as separate crates. So we could switch back to winit and also benefit whole community.
