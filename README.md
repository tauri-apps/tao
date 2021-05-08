# Tao

TODO

### Cargo Features

Tao provides the following features, which can be enabled in your `Cargo.toml` file:
* `serde`: Enables serialization/deserialization of certain types with [Serde](https://crates.io/crates/serde).
* `menu`: Enables system tray and more menu item variants on **Linux**. This flag is enabled by default.
  You can still create those types if you disable it. They just don't create the actual objects. We set this flag
  because some implementations require more installed packages.  Disable this if you don't want to install those
  additional packages.

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
sudo pacman -S gtk3 gtksourceview3 libappindicator-gtk3
```

#### Debian / Ubuntu:

```bash
sudo apt install libgtk-3-dev libgtksourceview-3.0-dev libappindicator3-dev
```

#### MacOS

To ensure compatibility with older MacOS systems, tao links to
CGDisplayCreateUUIDFromDisplayID through the CoreGraphics framework.
However, under certain setups this function is only available to be linked
through the newer ColorSync framework. So, tao provides the
`TAO_LINK_COLORSYNC` environment variable which can be set to `1` or `true` 
while compiling to enable linking via ColorSync.
