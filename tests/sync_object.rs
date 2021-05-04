#[allow(dead_code)]
fn needs_sync<T: Sync>() {}

#[test]
fn window_sync() {
  // ensures that `Window` implements `Sync`
  needs_sync::<tao::window::Window>();
}
