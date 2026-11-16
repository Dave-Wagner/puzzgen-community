//! Deterministic landmark placement and request identities, independent of Bevy.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
pub const STOPS: [(&str, &str); 3] = [
    ("Chime Grove", "sound-families"),
    ("Garden Post Office", "word-sorting"),
    ("Story Arbor", "sentence-assembly"),
];
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Trail {
    pub key: String,
    pub stops: Vec<Stop>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stop {
    pub cell: [usize; 2],
    pub seed: u64,
    #[serde(default)]
    pub restored: bool,
}
fn hash(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes.into_iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}
pub fn key(seed: &str, difficulty: &str, grid: &[Vec<i32>]) -> String {
    let fingerprint = hash(
        grid.iter()
            .flat_map(|r| r.iter().flat_map(|v| v.to_le_bytes()).chain([255])),
    );
    format!(
        "{}:{seed}:{}:{difficulty}:{fingerprint:016x}",
        seed.len(),
        difficulty.len()
    )
}
impl Trail {
    pub fn new(
        seed: &str,
        difficulty: &str,
        grid: &[Vec<i32>],
        start: [usize; 2],
        end: [usize; 2],
    ) -> Self {
        let key = key(seed, difficulty, grid);
        let empty = Self {
            key: key.clone(),
            stops: vec![],
        };
        let n = grid.len();
        let m = grid.first().map_or(0, Vec::len);
        if n == 0
            || n > 64
            || m == 0
            || m > 64
            || grid.iter().any(|r| r.len() != m)
            || start[0] >= n
            || start[1] >= m
            || end[0] >= n
            || end[1] >= m
            || grid[start[0]][start[1]] != 0
            || grid[end[0]][end[1]] != 0
        {
            return empty;
        }
        let mut parents = vec![vec![None; m]; n];
        let mut visited = vec![vec![false; m]; n];
        let mut queue = VecDeque::from([start]);
        visited[start[0]][start[1]] = true;
        while let Some([r, c]) = queue.pop_front() {
            for (dr, dc) in [(-1, 0), (0, 1), (1, 0), (0, -1)] {
                let rr = r as isize + dr;
                let cc = c as isize + dc;
                if rr >= 0 && cc >= 0 && (rr as usize) < n && (cc as usize) < m {
                    let (rr, cc) = (rr as usize, cc as usize);
                    if grid[rr][cc] == 0 && !visited[rr][cc] {
                        visited[rr][cc] = true;
                        parents[rr][cc] = Some([r, c]);
                        queue.push_back([rr, cc]);
                    }
                }
            }
        }
        if !visited[end[0]][end[1]] {
            return empty;
        }
        let mut path = vec![end];
        let mut at = end;
        while at != start {
            at = parents[at[0]][at[1]].unwrap();
            path.push(at);
        }
        path.reverse();
        if path.len() < 4 {
            return empty;
        }
        let stops = [0, (path.len() - 1) / 3, 2 * (path.len() - 1) / 3]
            .into_iter()
            .enumerate()
            .map(|(i, position)| Stop {
                cell: path[position],
                seed: hash(format!("lost-lantern-1:{key}:{i}").bytes()),
                restored: false,
            })
            .collect();
        Self { key, stops }
    }
    pub fn nearby(&self, cell: [usize; 2]) -> Option<usize> {
        self.stops
            .iter()
            .position(|s| s.cell[0].abs_diff(cell[0]) + s.cell[1].abs_diff(cell[1]) <= 1)
    }
    /// A reply may mutate only the exact expedition and stop that requested it.
    pub fn accepts(&self, key: &str, slot: usize, seed: u64) -> bool {
        self.key == key && self.stops.get(slot).is_some_and(|s| s.seed == seed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stops_are_reachable_distinct_and_replayable() {
        let grid = vec![vec![0; 7]];
        let a = Trail::new("42", "easy", &grid, [0, 0], [0, 6]);
        let b = Trail::new("42", "easy", &grid, [0, 0], [0, 6]);
        assert_eq!(a.key, b.key);
        assert_eq!(
            a.stops.iter().map(|s| (s.cell, s.seed)).collect::<Vec<_>>(),
            b.stops.iter().map(|s| (s.cell, s.seed)).collect::<Vec<_>>()
        );
        assert_eq!(
            a.stops
                .iter()
                .map(|s| s.cell)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3
        );
        assert!(a.stops.iter().all(|s| s.cell != [0, 6]));
        assert_eq!(a.nearby([0, 0]), Some(0));
        assert_eq!(a.nearby([9, 9]), None);
    }
    #[test]
    fn malformed_or_disconnected_mazes_produce_no_landmarks() {
        for grid in [vec![], vec![vec![0], vec![]], vec![vec![0, 1, 0, 0]]] {
            assert!(
                Trail::new("1", "easy", &grid, [0, 0], [0, 3])
                    .stops
                    .is_empty()
            );
        }
    }
    #[test]
    fn layout_changes_do_not_reuse_an_old_expedition_identity() {
        let first = vec![vec![0; 7]];
        let second = vec![vec![0; 8]];
        assert_ne!(key("42", "easy", &first), key("42", "easy", &second));
        assert_ne!(key("42", "easy", &first), key("42", "hard", &first));
    }
    #[test]
    fn stale_network_receipts_cannot_attach_to_another_expedition() {
        let first = Trail::new("42", "easy", &[vec![0; 7]], [0, 0], [0, 6]);
        let mut second = Trail::new("43", "easy", &[vec![0; 7]], [0, 0], [0, 6]);
        let seed = first.stops[0].seed;
        assert!(first.accepts(&first.key, 0, seed));
        assert!(!second.accepts(&first.key, 0, seed));
        assert!(!first.accepts(&first.key, 99, seed));
        assert!(!first.accepts(&first.key, 0, seed.wrapping_add(1)));
        second.stops[0].restored = true;
        let restored: Trail =
            serde_json::from_str(&serde_json::to_string(&second).unwrap()).unwrap();
        assert!(restored.stops[0].restored);
        assert!(!restored.accepts(&first.key, 0, seed));
    }
}
