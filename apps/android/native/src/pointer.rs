//! Preserve touch edges even when a whole tap occurs between render frames.
pub fn positions<T: Copy>(
    started: Option<T>,
    active: Option<T>,
    ended: Option<T>,
    cursor: Option<T>,
) -> (Option<T>, Option<T>) {
    (started.or(cursor), active.or(ended).or(cursor))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quick_tap_has_press_and_release_without_an_active_finger() {
        assert_eq!(
            positions(Some((10, 20)), None, Some((10, 20)), None),
            (Some((10, 20)), Some((10, 20)))
        );
    }
    #[test]
    fn fast_swipe_keeps_distinct_origin_and_endpoint() {
        assert_eq!(
            positions(Some((10, 20)), None, Some((90, 80)), None),
            (Some((10, 20)), Some((90, 80)))
        );
    }
    #[test]
    fn touch_coordinates_take_priority_over_a_stale_mouse_cursor() {
        assert_eq!(
            positions(Some(1), Some(2), None, Some(9)),
            (Some(1), Some(2))
        );
        assert_eq!(
            positions::<i32>(None, None, None, Some(9)),
            (Some(9), Some(9))
        );
    }
}
