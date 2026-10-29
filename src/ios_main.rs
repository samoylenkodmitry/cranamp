#![forbid(unsafe_code)]
#[cfg(target_os = "ios")]
fn main() {
    #[cfg(feature = "logging")]
    let _ = env_logger::try_init();
    cranamp::ios_entry_point();
}
#[cfg(not(target_os = "ios"))]
fn main() {}
