use std::time::{SystemTime, UNIX_EPOCH};

/// A different seed on every call, so power on shows different garbage.
pub fn random() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    nanos ^ u64::from(std::process::id()).rotate_left(32)
}
