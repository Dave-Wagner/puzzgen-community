//! Sample-level gate: Android callbacks silence audio even if rendering is suspended.
use std::sync::atomic::{AtomicBool, Ordering};
static ACTIVE: AtomicBool = AtomicBool::new(false);
static MUTED: AtomicBool = AtomicBool::new(false);
pub fn set_active(active: bool) {
    ACTIVE.store(active, Ordering::Release);
}
pub fn set_muted(muted: bool) {
    MUTED.store(muted, Ordering::Release);
}
pub fn allowed() -> bool {
    ACTIVE.load(Ordering::Acquire) && !MUTED.load(Ordering::Acquire)
}
pub fn sample(value: i16) -> i16 {
    apply(
        value,
        ACTIVE.load(Ordering::Acquire),
        MUTED.load(Ordering::Acquire),
    )
}
fn apply(value: i16, active: bool, muted: bool) -> i16 {
    if active && !muted { value } else { 0 }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locked_background_and_unfocused_are_silent() {
        for v in [i16::MIN, -1000, 0, 1000, i16::MAX] {
            assert_eq!(apply(v, false, false), 0);
            assert_eq!(apply(v, false, true), 0);
        }
    }
    #[test]
    fn mute_overrides_foreground() {
        assert_eq!(apply(12000, true, true), 0);
        assert_eq!(apply(12000, true, false), 12000);
    }
}
