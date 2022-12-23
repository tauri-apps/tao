use tao_macros::generate_package_name;

pub const PACKAGE: &str = generate_package_name!(com_example, tauri_app);

fn main() {}
