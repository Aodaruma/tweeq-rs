#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), slint::PlatformError> {
    tweeq_slint_demo_lib::run()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
