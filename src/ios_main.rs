#![forbid(unsafe_code)]

#[cfg(target_os = "ios")]
fn main() {
    cranamp::ios_entry_point();
}

// The `cranamp-ios` binary only runs on iOS. The `ios` feature can still be
// enabled on other targets (for example `--all-features` checks), where this
// binary has no entry point to call.
#[cfg(not(target_os = "ios"))]
fn main() {}
