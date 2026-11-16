//! Local generator throughput probe, not a network/server load test.
use puzzgen::puzzles::{Difficulty, GenerateRequest, PuzzleKind, generate};
use std::time::Instant;

fn main() {
    let count: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100)
        .clamp(1, 10_000);
    println!("{count} hard-preset puzzles per kind, sequential; generation only");
    println!("{:18} {:>12} {:>12}", "kind", "mean ms", "puzzles/sec");
    for kind in PuzzleKind::ALL {
        let start = Instant::now();
        for seed in 0..count {
            std::hint::black_box(
                generate(GenerateRequest {
                    kind,
                    seed: Some(seed),
                    difficulty: Difficulty::Hard,
                    include_solution: false,
                    content_version: None,
                })
                .unwrap(),
            );
        }
        let seconds = start.elapsed().as_secs_f64();
        println!(
            "{:18} {:>12.3} {:>12.1}",
            kind.slug(),
            seconds * 1_000.0 / count as f64,
            count as f64 / seconds
        );
    }
}
