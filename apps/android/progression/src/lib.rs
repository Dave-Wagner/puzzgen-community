//! Local-first sanctuary rewards. Layout undo can never roll back reward ownership.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub mod trail;
pub const SPOTS: usize = 8;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Garden {
    pub care_points: u64,
    pub lanterns: u32,
    pub layout: [bool; SPOTS],
    claimed: BTreeSet<String>,
}
impl Garden {
    pub fn complete_maze(&mut self, seed: &str, difficulty: &str) -> bool {
        self.complete_puzzle("maze", seed, difficulty)
    }
    pub fn complete_puzzle(&mut self, kind: &str, seed: &str, difficulty: &str) -> bool {
        if ![
            "sudoku",
            "mini-sudoku",
            "maze",
            "nonogram",
            "word-search",
            "sliding-puzzle",
            "lights-out",
            "minesweeper",
            "arithmetic",
            "cryptogram",
            "spoonerism",
            "anagram",
            "word-ladder",
            "compound-bridge",
            "word-sorting",
            "sequence-detective",
            "memory-pairs",
            "path-connections",
            "futoshiki",
            "sound-families",
            "sentence-assembly",
        ]
        .contains(&kind)
        {
            return false;
        }
        // Tuple encoding avoids delimiter collisions in puzzle identifiers.
        let legacy = format!("{}:{}{}:{}", seed.len(), seed, difficulty.len(), difficulty);
        // Preserve already-claimed maze identities from the first released garden build.
        let identity = if kind == "maze" {
            legacy
        } else {
            format!("puzzle{}:{}{}", kind.len(), kind, legacy)
        };
        if !self.claimed.insert(identity) {
            return false;
        }
        self.care_points = self.care_points.saturating_add(10);
        self.lanterns = self.lanterns.saturating_add(1);
        true
    }
    pub fn available(&self) -> u32 {
        self.lanterns
            .saturating_sub(self.layout.iter().filter(|&&placed| placed).count() as u32)
    }
    pub fn toggle(&mut self, spot: usize) -> Result<(), &'static str> {
        if spot >= SPOTS {
            return Err("Choose a garden spot.");
        }
        if !self.layout[spot] && self.available() == 0 {
            return Err("Your next puzzle can bring a lantern home.");
        }
        self.layout[spot] = !self.layout[spot];
        Ok(())
    }
    pub fn restore_layout(&mut self, layout: [bool; SPOTS]) -> bool {
        if layout.iter().filter(|&&placed| placed).count() as u32 > self.lanterns {
            return false;
        }
        self.layout = layout;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_puzzle_kind_contributes_without_seed_collisions() {
        let mut garden = Garden::default();
        for kind in [
            "sudoku",
            "mini-sudoku",
            "maze",
            "nonogram",
            "word-search",
            "sliding-puzzle",
            "lights-out",
            "minesweeper",
            "arithmetic",
            "cryptogram",
        ] {
            assert!(garden.complete_puzzle(kind, "42", "easy"));
            assert!(!garden.complete_puzzle(kind, "42", "easy"));
        }
        assert_eq!((garden.care_points, garden.lanterns), (100, 10));
        assert!(!garden.complete_puzzle("unknown", "42", "easy"));
    }
    #[test]
    fn existing_maze_claim_survives_shared_reward_upgrade() {
        let mut garden: Garden =
            serde_json::from_str(r#"{"care_points":10,"lanterns":1,"claimed":["2:424:easy"]}"#)
                .unwrap();
        assert!(!garden.complete_puzzle("maze", "42", "easy"));
        assert!(garden.complete_puzzle("sudoku", "42", "easy"));
        assert_eq!((garden.care_points, garden.lanterns), (20, 2));
    }
    #[test]
    fn replay_and_reload_do_not_duplicate_rewards() {
        let mut g = Garden::default();
        assert!(g.complete_maze("42", "easy"));
        let mut g: Garden = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert!(!g.complete_maze("42", "easy"));
        assert_eq!((g.care_points, g.lanterns), (10, 1));
        assert!(g.complete_maze("43", "easy"));
        assert_eq!((g.care_points, g.lanterns), (20, 2));
    }
    #[test]
    fn placement_is_limited_to_earned_inventory() {
        let mut g = Garden::default();
        assert!(g.toggle(0).is_err());
        g.complete_maze("1", "easy");
        assert!(g.toggle(0).is_ok());
        assert_eq!(g.available(), 0);
        assert!(g.toggle(1).is_err());
        assert!(g.toggle(0).is_ok());
        assert_eq!(g.available(), 1);
        assert!(g.toggle(SPOTS).is_err());
    }
    #[test]
    fn undo_preserves_currency_and_reward_ledger() {
        let mut g = Garden::default();
        g.complete_maze("1", "easy");
        let before = g.layout;
        g.toggle(2).unwrap();
        assert!(g.restore_layout(before));
        assert_eq!((g.care_points, g.available()), (10, 1));
        assert!(!g.complete_maze("1", "easy"));
        assert!(!g.restore_layout([true; SPOTS]));
    }
    #[test]
    fn old_empty_save_and_distinct_difficulties_are_supported() {
        let mut g: Garden = serde_json::from_str("{}").unwrap();
        assert!(g.complete_maze("1", "easy"));
        assert!(g.complete_maze("1", "hard"));
        assert_eq!(g.lanterns, 2);
    }
    #[test]
    fn layout_survives_reload() {
        let mut g = Garden::default();
        g.complete_maze("5", "easy");
        g.toggle(5).unwrap();
        let restored: Garden = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert!(restored.layout[5]);
        assert_eq!(restored.available(), 0);
    }
    #[test]
    fn lantern_visits_earn_shared_inventory_once_even_after_reload() {
        let mut garden = Garden::default();
        for (_, kind) in trail::STOPS {
            assert!(garden.complete_puzzle(kind, "42", "easy"));
        }
        assert_eq!((garden.care_points, garden.available()), (30, 3));
        garden.toggle(0).unwrap();
        let mut restored: Garden =
            serde_json::from_str(&serde_json::to_string(&garden).unwrap()).unwrap();
        for (_, kind) in trail::STOPS {
            assert!(!restored.complete_puzzle(kind, "42", "easy"));
        }
        assert_eq!((restored.care_points, restored.available()), (30, 2));
        assert!(restored.layout[0]);
    }
}
