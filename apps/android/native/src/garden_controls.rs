//! Small movement decisions shared by the phone prototype and standalone tests.
use crate::walk_physics;

pub fn is_tap(max_travel_squared: f32, seconds: f32) -> bool {
    max_travel_squared.is_finite()
        && seconds.is_finite()
        && (0.0..=64.).contains(&max_travel_squared)
        && (0.0..=0.45).contains(&seconds)
}

pub fn destination(grid: &[Vec<i32>], from: [f32; 2], to: [f32; 2]) -> Option<[f32; 2]> {
    if !from.into_iter().chain(to).all(f32::is_finite) {
        return None;
    }
    let dx = to[0] - from[0];
    let dz = to[1] - from[1];
    let length = dx.hypot(dz);
    if !(0.12..=3.0).contains(&length) {
        return None;
    }
    // Only a short, unobstructed line is accepted. Never pathfind around a hedge.
    let steps = (length / 0.08).ceil() as usize;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        if !walk_physics::clear(grid, from[0] + dx * t, from[1] + dz * t) {
            return None;
        }
    }
    Some(to)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grid() -> Vec<Vec<i32>> {
        vec![
            vec![1; 6],
            vec![1, 0, 0, 1, 0, 1],
            vec![1, 0, 0, 1, 0, 1],
            vec![1, 0, 0, 0, 0, 1],
            vec![1; 6],
        ]
    }
    #[test]
    fn nearby_clear_ground_is_accepted() {
        assert_eq!(destination(&grid(), [3., 3.], [4.8, 3.]), Some([4.8, 3.]));
    }
    #[test]
    fn walls_and_corners_are_not_shortcuts() {
        assert!(destination(&grid(), [5., 3.], [7.5, 3.]).is_none());
        assert!(destination(&grid(), [5., 5.], [7., 7.]).is_none());
    }
    #[test]
    fn invalid_far_and_zero_destinations_are_rejected() {
        for to in [[3., 3.], [3., 7.], [f32::NAN, 3.], [f32::INFINITY, 3.]] {
            assert!(destination(&grid(), [3., 3.], to).is_none());
        }
    }
    #[test]
    fn looking_and_long_holds_are_not_walk_taps() {
        assert!(is_tap(16., 0.15));
        assert!(!is_tap(81., 0.15));
        assert!(!is_tap(0., 0.8));
    }
}
