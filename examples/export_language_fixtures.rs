//! Export deterministic offline teaching fixtures for both language corpora.
use puzzgen::puzzles::{Difficulty, GenerateRequest, PuzzleKind, generate};
fn main() {
    let mut fixtures = Vec::new();
    for version in ["1", "2"] {
        for kind in [
            PuzzleKind::Spoonerism,
            PuzzleKind::Anagram,
            PuzzleKind::WordLadder,
            PuzzleKind::CompoundBridge,
        ] {
            fixtures.push(
                generate(GenerateRequest {
                    kind,
                    seed: Some(42),
                    difficulty: Difficulty::Medium,
                    include_solution: true,
                    content_version: Some(version.into()),
                })
                .unwrap(),
            );
        }
    }
    println!("{}", serde_json::to_string_pretty(&fixtures).unwrap());
}
