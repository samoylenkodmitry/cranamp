#![forbid(unsafe_code)]
#[cfg(target_os = "ios")]
fn main() {
    cranamp::ios_entry_point();
}
#[cfg(not(target_os = "ios"))]
fn main() {}
