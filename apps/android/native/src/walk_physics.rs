//! Horizontal capsule collision against maze hedges; independent of rendering.
pub const TILE: f32 = 2.;
pub const RADIUS: f32 = 0.27;
pub fn clear(grid: &[Vec<i32>], x: f32, z: f32) -> bool {
    for r in ((z - RADIUS) / TILE).floor() as isize..=((z + RADIUS) / TILE).floor() as isize {
        for c in ((x - RADIUS) / TILE).floor() as isize..=((x + RADIUS) / TILE).floor() as isize {
            if r < 0
                || c < 0
                || grid
                    .get(r as usize)
                    .and_then(|row| row.get(c as usize))
                    .is_none_or(|v| *v != 0)
            {
                let cx = x.clamp(c as f32 * TILE, (c + 1) as f32 * TILE);
                let cz = z.clamp(r as f32 * TILE, (r + 1) as f32 * TILE);
                if (x - cx).powi(2) + (z - cz).powi(2) < RADIUS * RADIUS {
                    return false;
                }
            }
        }
    }
    true
}
pub fn advance(grid: &[Vec<i32>], position: [f32; 2], delta: [f32; 2]) -> [f32; 2] {
    // Substeps prevent tunneling on slow frames; separate axes allow sliding.
    let steps = ((delta[0].abs().max(delta[1].abs()) / 0.10).ceil() as usize).max(1);
    let mut p = position;
    for _ in 0..steps {
        let x = p[0] + delta[0] / steps as f32;
        if clear(grid, x, p[1]) {
            p[0] = x;
        }
        let z = p[1] + delta[1] / steps as f32;
        if clear(grid, p[0], z) {
            p[1] = z;
        }
    }
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    fn grid() -> Vec<Vec<i32>> {
        vec![vec![1, 1, 1, 1], vec![1, 0, 0, 1], vec![1, 1, 1, 1]]
    }
    #[test]
    fn walks_through_corridors_and_stops_at_hedges() {
        let g = grid();
        let p = advance(&g, [3., 3.], [20., 0.]);
        assert!(p[0] < 6. - RADIUS + 0.01);
        assert!(p[0] > 5.5);
        assert!(clear(&g, p[0], p[1]));
    }
    #[test]
    fn cannot_cut_corners_or_leave_the_map() {
        let g = grid();
        let p = advance(&g, [3., 3.], [-20., -20.]);
        assert!(p[0] >= 2. + RADIUS - 0.01 && p[1] >= 2. + RADIUS - 0.01);
        assert!(clear(&g, p[0], p[1]));
    }
    #[test]
    fn slides_along_a_wall_instead_of_getting_stuck() {
        let g = grid();
        let p = advance(&g, [3., 2.3], [1., -1.]);
        assert!(p[0] > 3.9);
        assert!(p[1] >= 2. + RADIUS - 0.01);
    }
}
