use tao_macros::android_fn;

struct JNIEnv;
struct JClass;

android_fn![com_example, tao_app, SomeClass, add, [i32, i32], i32];
unsafe fn add(_env: JNIEnv, _class: JClass, a: i32, b: i32) -> i32 {
  a + b
}

android_fn!(com_example, tao_app, SomeClass, add2, [i32, i32]);
unsafe fn add2(_env: JNIEnv, _class: JClass, a: i32, b: i32) {
  let _ = a + b;
}

fn __store_package_name__() {}

android_fn!(
  com_example,
  tao_app,
  SomeClass,
  add3,
  [i32, i32],
  __VOID__,
  [setup, main],
  __store_package_name__,
);
unsafe fn add3(_env: JNIEnv, _class: JClass, a: i32, b: i32, _setup: fn(), _main: fn()) {
  let _ = a + b;
}

fn setup() {}
fn main() {}
