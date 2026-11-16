use puzzgen::puzzles::{Difficulty, GenerateRequest, generate};
fn main() {
    let fixtures = puzzgen::discovery::KINDS.map(|kind| {
        generate(GenerateRequest {
            kind,
            seed: Some(42),
            difficulty: Difficulty::Medium,
            include_solution: true,
            content_version: None,
        })
        .unwrap()
    });
    println!("{}", serde_json::to_string_pretty(&fixtures).unwrap());
}
