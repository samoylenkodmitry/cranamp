//! What Cranamp asks of a device. The Android manifest and the Apple usage
//! descriptions are written from this one list.

use cranpose_capabilities::{declare, Use};

fn main() {
    declare(&[Use::media(), Use::update(), Use::network()]).emit();
}
