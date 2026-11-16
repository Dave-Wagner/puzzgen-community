//! Pure, seeded puzzle generation. Bump GENERATOR_VERSION if algorithms change.
use rand::{Rng, SeedableRng, seq::SliceRandom};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::VecDeque, str::FromStr};

pub const GENERATOR_VERSION: &str = "2";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    #[default]
    Medium,
    Hard,
}

impl Difficulty {
    fn select<T>(self, easy: T, medium: T, hard: T) -> T {
        match self {
            Self::Easy => easy,
            Self::Medium => medium,
            Self::Hard => hard,
        }
    }
    fn slug(self) -> &'static str {
        self.select("easy", "medium", "hard")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PuzzleKind {
    Sudoku,
    MiniSudoku,
    Maze,
    Nonogram,
    WordSearch,
    SlidingPuzzle,
    LightsOut,
    Minesweeper,
    Arithmetic,
    Cryptogram,
    Spoonerism,
    Anagram,
    WordLadder,
    CompoundBridge,
    WordSorting,
    SequenceDetective,
    MemoryPairs,
    PathConnections,
    Futoshiki,
    SoundFamilies,
    SentenceAssembly,
}

impl PuzzleKind {
    pub const ALL: [Self; 21] = [
        Self::Sudoku,
        Self::MiniSudoku,
        Self::Maze,
        Self::Nonogram,
        Self::WordSearch,
        Self::SlidingPuzzle,
        Self::LightsOut,
        Self::Minesweeper,
        Self::Arithmetic,
        Self::Cryptogram,
        Self::Spoonerism,
        Self::Anagram,
        Self::WordLadder,
        Self::CompoundBridge,
        Self::WordSorting,
        Self::SequenceDetective,
        Self::MemoryPairs,
        Self::PathConnections,
        Self::Futoshiki,
        Self::SoundFamilies,
        Self::SentenceAssembly,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Sudoku => "sudoku",
            Self::MiniSudoku => "mini-sudoku",
            Self::Maze => "maze",
            Self::Nonogram => "nonogram",
            Self::WordSearch => "word-search",
            Self::SlidingPuzzle => "sliding-puzzle",
            Self::LightsOut => "lights-out",
            Self::Minesweeper => "minesweeper",
            Self::Arithmetic => "arithmetic",
            Self::Cryptogram => "cryptogram",
            Self::Spoonerism => "spoonerism",
            Self::Anagram => "anagram",
            Self::WordLadder => "word-ladder",
            Self::CompoundBridge => "compound-bridge",
            Self::WordSorting => "word-sorting",
            Self::SequenceDetective => "sequence-detective",
            Self::MemoryPairs => "memory-pairs",
            Self::PathConnections => "path-connections",
            Self::Futoshiki => "futoshiki",
            Self::SoundFamilies => "sound-families",
            Self::SentenceAssembly => "sentence-assembly",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Sudoku => "Sudoku",
            Self::MiniSudoku => "Mini Sudoku",
            Self::Maze => "Perfect maze",
            Self::Nonogram => "Nonogram",
            Self::WordSearch => "Word search",
            Self::SlidingPuzzle => "Sliding puzzle",
            Self::LightsOut => "Lights Out",
            Self::Minesweeper => "Minesweeper",
            Self::Arithmetic => "Arithmetic",
            Self::Cryptogram => "Cryptogram",
            Self::Spoonerism => "Spoonerism",
            Self::Anagram => "Anagram",
            Self::WordLadder => "Word ladder",
            Self::CompoundBridge => "Compound bridge",
            Self::WordSorting => "Word sorting",
            Self::SequenceDetective => "Sequence detective",
            Self::MemoryPairs => "Memory pairs",
            Self::PathConnections => "Path connections",
            Self::Futoshiki => "Futoshiki",
            Self::SoundFamilies => "Sound families",
            Self::SentenceAssembly => "Sentence assembly",
        }
    }

    fn rules(self) -> &'static str {
        match self {
            Self::WordSorting => {
                "Assign each item ID to its curated category ID. Use the everyday meanings supplied by the category labels."
            }
            Self::SequenceDetective => {
                "Find the missing integer using the named sequence family; do not invent a different rule."
            }
            Self::MemoryPairs => {
                "Pair every card ID exactly once with another card having the same face. Faces are public construction data, not an anti-cheat boundary."
            }
            Self::PathConnections => {
                "Rotate tiles to connect every tile to the source. Every port must meet an opposite port; no boundary leaks are allowed. Bits: north=1 east=2 south=4 west=8."
            }
            Self::Futoshiki => {
                "Fill zeros with 1–4 exactly once per row and column. Preserve givens and satisfy all adjacent inequalities."
            }
            Self::SoundFamilies => {
                "Select every candidate ID that rhymes with the anchor in general US English. Order is ignored; duplicate IDs are rejected."
            }
            Self::SentenceAssembly => {
                "Use every tile ID once. Build the noun phrase (the adjective animal), then action and adverb. The adverb may go at the end, before the verb, or at the beginning."
            }
            Self::Sudoku | Self::MiniSudoku => {
                "Fill zero cells so every row, column and box contains each digit exactly once. Difficulty controls clue removal, not a human solving-technique rating."
            }
            Self::Maze => {
                "Find an orthogonal path from start to end. A grid value of 1 is a wall; 0 is open. Coordinates are zero-based [row,column]."
            }
            Self::Nonogram => {
                "Fill cells to match each row and column's consecutive filled-run lengths. Empty clues mean an empty line. Difficulty controls board dimensions."
            }
            Self::WordSearch => {
                "Locate every word in a straight horizontal, vertical or diagonal line, in either direction. Other accidental occurrences are allowed."
            }
            Self::SlidingPuzzle => {
                "Slide adjacent numbered tiles into the empty zero cell until numbers are in row-major order with zero last. Move directions describe the EMPTY cell's movement. Difficulty controls scramble length, not optimal solution distance."
            }
            Self::LightsOut => {
                "Pressing a cell toggles it and its orthogonal neighbors. Turn every light off. A grid value of 1 means on. Multiple valid press patterns can exist."
            }
            Self::Minesweeper => {
                "Flag every mine using the revealed neighboring-mine counts. The start and its neighbors are mine-free. This is a safe-start board generator; no-guess solvability is not guaranteed."
            }
            Self::Arithmetic => {
                "Evaluate the integer expression. Parentheses apply before multiplication, which applies before addition and subtraction."
            }
            Self::Spoonerism => {
                "Swap the opening consonant sounds of the source words to match the clue. Curated general US English; spelling alone is not the rule. Answer with the target phrase."
            }
            Self::Anagram => {
                "Rearrange all supplied letters into the clued word. Hard mode accepts any listed vocabulary word with exactly those letters. ASCII case and repeated whitespace are ignored."
            }
            Self::WordLadder => {
                "Change one letter at a time from start to end using only allowed_words. Submit the full path, including endpoints, with no repeated words and at most max_steps changes."
            }
            Self::CompoundBridge => {
                "Find the middle word forming two curated compounds: left+answer and answer+right. Compounds may conventionally be closed, spaced or hyphenated. Answer with the middle word."
            }
            Self::Cryptogram => {
                "Decode the monoalphabetic substitution. The same ciphertext letter always represents the same plaintext letter. Punctuation and spaces are preserved. The selected phrase is the accepted answer; linguistic uniqueness is not guaranteed."
            }
        }
    }

    fn uniqueness(self) -> &'static str {
        match self {
            Self::WordSorting => "curated-category-assignment",
            Self::SequenceDetective => "unique-within-stated-family",
            Self::MemoryPairs => "multiple-pair-orders-accepted",
            Self::PathConnections => "multiple-valid-orientations-accepted",
            Self::Futoshiki => "verified-unique",
            Self::SoundFamilies => "curated-pronunciation-family",
            Self::SentenceAssembly => "three-controlled-grammar-orders-accepted",
            Self::Sudoku | Self::MiniSudoku | Self::Nonogram => "verified-unique",
            Self::Maze => "perfect-maze-unique-simple-path",
            Self::Arithmetic => "unique-integer-result",
            Self::WordLadder => "multiple-valid-solutions-accepted",
            Self::Spoonerism | Self::CompoundBridge => {
                "curated-answer-only-linguistic-uniqueness-not-guaranteed"
            }
            Self::Anagram => "curated-answer-or-hard-mode-vocabulary-anagrams",
            Self::WordSearch | Self::LightsOut | Self::SlidingPuzzle => {
                "multiple-valid-solutions-accepted"
            }
            Self::Minesweeper => "safe-start-only-no-guess-not-guaranteed",
            Self::Cryptogram => "selected-plaintext-only-linguistic-uniqueness-not-guaranteed",
        }
    }
}

impl FromStr for PuzzleKind {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.slug() == value)
            .ok_or_else(|| format!("Unknown puzzle kind: {value}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateRequest {
    pub kind: PuzzleKind,
    pub seed: Option<u64>,
    #[serde(default)]
    pub difficulty: Difficulty,
    #[serde(default)]
    pub include_solution: bool,
    /// Language corpus replay selector; omitted means latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Puzzle {
    pub id: String,
    pub generator_version: String,
    pub kind: PuzzleKind,
    /// Decimal string avoids precision loss in JavaScript API clients.
    pub seed: String,
    pub difficulty: Difficulty,
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solution: Option<Value>,
    pub rules: String,
    pub uniqueness: String,
}

pub fn catalog() -> Value {
    json!({"generator_version": GENERATOR_VERSION, "kinds": PuzzleKind::ALL.map(|kind| {
        let answer = match kind {
            PuzzleKind::WordSorting => "Object mapping item IDs to category IDs",
            PuzzleKind::SequenceDetective => "Integer JSON number",
            PuzzleKind::MemoryPairs => "Array of two-card ID arrays",
            PuzzleKind::PathConnections => "Grid of oriented integer port masks (1..15)",
            PuzzleKind::Futoshiki => "Completed 4x4 integer grid",
            PuzzleKind::SoundFamilies => "Array of matching candidate IDs",
            PuzzleKind::SentenceAssembly => "Array of tile IDs in sentence order",
            PuzzleKind::Sudoku | PuzzleKind::MiniSudoku => "Completed numeric grid: [[1,2,...],...]",
            PuzzleKind::Maze => "Path including start and end: [[row,column],...]",
            PuzzleKind::Nonogram => "Binary grid: [[0,1,...],...]",
            PuzzleKind::WordSearch => "[{\"word\":\"RUST\",\"cells\":[[row,column],...]}]",
            PuzzleKind::SlidingPuzzle => "{\"moves\":[\"up\",\"down\",\"left\",\"right\",...]}",
            PuzzleKind::LightsOut | PuzzleKind::Minesweeper => "Cell coordinates: [[row,column],...]",
            PuzzleKind::Arithmetic => "Integer JSON number",
            PuzzleKind::Spoonerism | PuzzleKind::Anagram | PuzzleKind::CompoundBridge => "JSON string; ASCII case and repeated whitespace ignored",
            PuzzleKind::WordLadder => "Array of word strings including start and end",
            PuzzleKind::Cryptogram => "Plaintext JSON string; case and repeated whitespace are ignored",
        };
        json!({"kind":kind,"name":kind.name(),"description":kind.rules(),"rules":kind.rules(),
            "answer_format":answer,"uniqueness":kind.uniqueness()})
    })})
}

pub fn generate(request: GenerateRequest) -> Result<Puzzle, String> {
    let language = matches!(
        request.kind,
        PuzzleKind::Spoonerism
            | PuzzleKind::Anagram
            | PuzzleKind::WordLadder
            | PuzzleKind::CompoundBridge
    );
    let content_version = request.content_version.as_deref().unwrap_or("2");
    if request.content_version.is_some() && (!language || !["1", "2"].contains(&content_version)) {
        return Err(
            "content_version must be 1 or 2 and is only supported for language puzzles".into(),
        );
    }
    let seed = request.seed.unwrap_or_else(rand::random);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let (data, solution) = match request.kind {
        kind @ (PuzzleKind::WordSorting
        | PuzzleKind::SequenceDetective
        | PuzzleKind::MemoryPairs
        | PuzzleKind::PathConnections
        | PuzzleKind::Futoshiki
        | PuzzleKind::SoundFamilies
        | PuzzleKind::SentenceAssembly) => {
            crate::discovery::generate(kind, &mut rng, request.difficulty)
        }
        PuzzleKind::Sudoku => sudoku(&mut rng, 3, request.difficulty),
        PuzzleKind::MiniSudoku => sudoku(&mut rng, 2, request.difficulty),
        PuzzleKind::Maze => maze(&mut rng, request.difficulty),
        PuzzleKind::Nonogram => nonogram(&mut rng, request.difficulty),
        PuzzleKind::WordSearch => word_search(&mut rng, request.difficulty),
        PuzzleKind::SlidingPuzzle => sliding(&mut rng, request.difficulty),
        PuzzleKind::LightsOut => lights(&mut rng, request.difficulty),
        PuzzleKind::Minesweeper => minesweeper(&mut rng, request.difficulty),
        PuzzleKind::Arithmetic => arithmetic(&mut rng, request.difficulty),
        PuzzleKind::Cryptogram => cryptogram(&mut rng, request.difficulty),
        kind @ (PuzzleKind::Spoonerism
        | PuzzleKind::Anagram
        | PuzzleKind::WordLadder
        | PuzzleKind::CompoundBridge) => {
            crate::language::generate_version(kind, &mut rng, request.difficulty, content_version)
        }
    };
    Ok(Puzzle {
        id: format!(
            "v{GENERATOR_VERSION}-{}-{seed}-{}{}",
            request.kind.slug(),
            request.difficulty.slug(),
            if language && content_version != "1" {
                format!("-content{content_version}")
            } else {
                String::new()
            }
        ),
        generator_version: GENERATOR_VERSION.into(),
        kind: request.kind,
        seed: seed.to_string(),
        difficulty: request.difficulty,
        data,
        solution: request.include_solution.then_some(solution),
        rules: request.kind.rules().into(),
        uniqueness: request.kind.uniqueness().into(),
    })
}

/// Regenerates the original challenge rather than trusting client-provided clues.
pub fn verify(mut request: GenerateRequest, answer: Value) -> Result<bool, String> {
    if request.seed.is_none() {
        return Err("Verification requires the original seed".into());
    }
    request.include_solution = true;
    let original = request.clone();
    let puzzle = generate(request)?;
    let solution = puzzle
        .solution
        .as_ref()
        .ok_or("Missing generated solution")?;
    match puzzle.kind {
        kind @ (PuzzleKind::WordSorting
        | PuzzleKind::SequenceDetective
        | PuzzleKind::MemoryPairs
        | PuzzleKind::PathConnections
        | PuzzleKind::Futoshiki
        | PuzzleKind::SoundFamilies
        | PuzzleKind::SentenceAssembly) => Ok(crate::discovery::verify(
            kind,
            &puzzle.data,
            solution,
            &answer,
        )),
        kind @ (PuzzleKind::Spoonerism
        | PuzzleKind::Anagram
        | PuzzleKind::WordLadder
        | PuzzleKind::CompoundBridge) => Ok(crate::language::verify(
            kind,
            &puzzle.data,
            solution,
            &answer,
        )),
        PuzzleKind::Sudoku
        | PuzzleKind::MiniSudoku
        | PuzzleKind::Nonogram
        | PuzzleKind::Arithmetic => Ok(&answer == solution),
        PuzzleKind::Cryptogram => {
            let normalize = |text: &str| {
                text.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_ascii_uppercase()
            };
            Ok(answer.as_str().is_some_and(|text| {
                normalize(text) == normalize(solution.as_str().unwrap_or_default())
            }))
        }
        PuzzleKind::Maze => {
            let path: Vec<[usize; 2]> = parse_answer(answer)?;
            let grid: Vec<Vec<u8>> = parse_answer(puzzle.data["grid"].clone())?;
            let start: [usize; 2] = parse_answer(puzzle.data["start"].clone())?;
            let end: [usize; 2] = parse_answer(puzzle.data["end"].clone())?;
            if path.first() != Some(&start)
                || path.last() != Some(&end)
                || path.len() > grid.len() * grid[0].len() * 4
            {
                return Ok(false);
            }
            let legal_steps = path
                .windows(2)
                .all(|pair| pair[0][0].abs_diff(pair[1][0]) + pair[0][1].abs_diff(pair[1][1]) == 1);
            if !legal_steps {
                return Ok(false);
            }
            if path
                .iter()
                .all(|[r, c]| grid.get(*r).and_then(|row| row.get(*c)) == Some(&0))
            {
                return Ok(true);
            }
            let mut rng = ChaCha8Rng::seed_from_u64(original.seed.expect("seed checked above"));
            let (old_data, _) = maze_v1(&mut rng, original.difficulty);
            let old_grid: Vec<Vec<u8>> = parse_answer(old_data["grid"].clone())?;
            Ok(path
                .iter()
                .all(|[r, c]| old_grid.get(*r).and_then(|row| row.get(*c)) == Some(&0)))
        }
        PuzzleKind::LightsOut => {
            let presses: Vec<[usize; 2]> = parse_answer(answer)?;
            let mut grid: Vec<Vec<u8>> = parse_answer(puzzle.data["grid"].clone())?;
            let n = grid.len();
            if presses.len() > n * n * 4 {
                return Ok(false);
            }
            for [r, c] in presses {
                if r >= n || c >= n {
                    return Ok(false);
                }
                toggle(&mut grid, r, c);
            }
            Ok(grid.iter().flatten().all(|cell| *cell == 0))
        }
        PuzzleKind::Minesweeper => {
            let mut submitted: Vec<[usize; 2]> = parse_answer(answer)?;
            let mut expected: Vec<[usize; 2]> = parse_answer(solution.clone())?;
            submitted.sort_unstable();
            expected.sort_unstable();
            Ok(submitted == expected)
        }
        PuzzleKind::SlidingPuzzle => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Moves {
                moves: Vec<String>,
            }
            let submitted: Moves = parse_answer(answer)?;
            if submitted.moves.len() > 10_000 {
                return Ok(false);
            }
            let mut grid: Vec<Vec<u8>> = parse_answer(puzzle.data["grid"].clone())?;
            for direction in submitted.moves {
                if !slide(&mut grid, &direction) {
                    return Ok(false);
                }
            }
            Ok(grid == solved_sliding(grid.len()))
        }
        PuzzleKind::WordSearch => verify_words(&puzzle.data, answer),
    }
}

fn parse_answer<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value)
        .map_err(|_| "Answer has the wrong shape; see catalog answer_format".into())
}

fn sudoku(rng: &mut ChaCha8Rng, box_size: usize, difficulty: Difficulty) -> (Value, Value) {
    let n = box_size * box_size;
    let mut digits = (1..=n as u8).collect::<Vec<_>>();
    digits.shuffle(rng);
    let mut bands = (0..box_size).collect::<Vec<_>>();
    bands.shuffle(rng);
    let mut stacks = (0..box_size).collect::<Vec<_>>();
    stacks.shuffle(rng);
    let mut rows = Vec::new();
    let mut cols = Vec::new();
    for band in bands {
        let mut local = (0..box_size).collect::<Vec<_>>();
        local.shuffle(rng);
        rows.extend(local.into_iter().map(|r| band * box_size + r));
    }
    for stack in stacks {
        let mut local = (0..box_size).collect::<Vec<_>>();
        local.shuffle(rng);
        cols.extend(local.into_iter().map(|c| stack * box_size + c));
    }
    let solution: Vec<Vec<u8>> = rows
        .into_iter()
        .map(|r| {
            cols.iter()
                .map(|c| digits[(r * box_size + r / box_size + c) % n])
                .collect()
        })
        .collect();
    let mut grid = solution.clone();
    let target = if n == 9 {
        difficulty.select(42, 34, 28)
    } else {
        difficulty.select(9, 7, 6)
    };
    let mut positions = (0..n * n).collect::<Vec<_>>();
    positions.shuffle(rng);
    let mut clues = n * n;
    for index in positions {
        if clues <= target {
            break;
        }
        let (r, c) = (index / n, index % n);
        let previous = grid[r][c];
        grid[r][c] = 0;
        let mut budget = 30_000;
        let count = sudoku_count(&mut grid, box_size, 2, &mut budget);
        if count == 1 && budget > 0 {
            clues -= 1;
        } else {
            grid[r][c] = previous;
        }
    }
    (
        json!({"grid":grid,"box_size":box_size,"clue_count":clues}),
        json!(solution),
    )
}

fn sudoku_count(grid: &mut [Vec<u8>], b: usize, limit: usize, budget: &mut usize) -> usize {
    if *budget == 0 {
        return 0;
    }
    *budget -= 1;
    let n = grid.len();
    let mut best: Option<(usize, usize, Vec<u8>)> = None;
    for r in 0..n {
        for c in 0..n {
            if grid[r][c] != 0 {
                continue;
            }
            let choices: Vec<u8> = (1..=n as u8)
                .filter(|digit| {
                    !grid[r].contains(digit)
                        && !(0..n).any(|row| grid[row][c] == *digit)
                        && !(r / b * b..r / b * b + b).any(|row| {
                            (c / b * b..c / b * b + b).any(|col| grid[row][col] == *digit)
                        })
                })
                .collect();
            if choices.is_empty() {
                return 0;
            }
            if best
                .as_ref()
                .is_none_or(|(_, _, old)| choices.len() < old.len())
            {
                best = Some((r, c, choices));
            }
        }
    }
    let Some((r, c, choices)) = best else {
        return 1;
    };
    let mut count = 0;
    for digit in choices {
        grid[r][c] = digit;
        count += sudoku_count(grid, b, limit - count, budget);
        grid[r][c] = 0;
        if count >= limit || *budget == 0 {
            break;
        }
    }
    count
}

fn maze(rng: &mut ChaCha8Rng, difficulty: Difficulty) -> (Value, Value) {
    let cells = difficulty.select(5, 8, 12);
    let n = cells * 2 + 1;
    let mut best = None;
    let mut best_score = 0;
    for _ in 0..64 {
        let mut grid = vec![vec![1u8; n]; n];
        grid[1][1] = 0;
        let mut frontier = vec![(1usize, 1usize, 1usize, 3usize), (1, 1, 3, 1)];
        while !frontier.is_empty() {
            // Mix short branching growth with winding corridors; pure Prim
            // growth alone can produce too direct a corner-to-corner route.
            let index = if rng.random_bool(0.65) {
                frontier.len() - 1
            } else {
                rng.random_range(0..frontier.len())
            };
            let (r, c, rr, cc) = frontier.swap_remove(index);
            if grid[rr][cc] == 0 {
                continue;
            }
            grid[(r + rr) / 2][(c + cc) / 2] = 0;
            grid[rr][cc] = 0;
            let mut directions = [(-2isize, 0), (2, 0), (0, -2), (0, 2)];
            directions.shuffle(rng);
            for (dr, dc) in directions {
                let nr = rr as isize + dr;
                let nc = cc as isize + dc;
                if nr > 0
                    && nc > 0
                    && nr < (n - 1) as isize
                    && nc < (n - 1) as isize
                    && grid[nr as usize][nc as usize] == 1
                {
                    frontier.push((rr, cc, nr as usize, nc as usize));
                }
            }
        }
        let path = maze_route(&grid);
        let (forks, decoys) = maze_decisions(&grid, &path);
        let score = path.len() + forks * 8 + decoys * 12;
        let acceptable = path.len() >= difficulty.select(21, 37, 57)
            && forks >= difficulty.select(3, 5, 8)
            && decoys >= difficulty.select(2, 3, 5);
        if acceptable {
            return (
                json!({"grid":grid,"start":[1,1],"end":[n-2,n-2]}),
                json!(path),
            );
        }
        if best.is_none() || score > best_score {
            best_score = score;
            best = Some((
                json!({"grid":grid,"start":[1,1],"end":[n-2,n-2]}),
                json!(path),
            ));
        }
    }
    best.expect("bounded maze generation always produces a connected candidate")
}

fn maze_route(grid: &[Vec<u8>]) -> Vec<[usize; 2]> {
    let n = grid.len();
    let start = [1, 1];
    let end = [n - 2, n - 2];
    let mut parent = vec![vec![None; n]; n];
    parent[1][1] = Some(start);
    let mut queue = VecDeque::from([start]);
    while let Some([r, c]) = queue.pop_front() {
        for [rr, cc] in neighbors(r, c, n, n) {
            if grid[rr][cc] == 0 && parent[rr][cc].is_none() {
                parent[rr][cc] = Some([r, c]);
                queue.push_back([rr, cc]);
            }
        }
    }
    let mut path = vec![end];
    let mut current = end;
    while current != start {
        current = parent[current[0]][current[1]].expect("maze is connected");
        path.push(current);
    }
    path.reverse();
    path
}

/// Count actual choices on the winning route, and wrong turns at least four
/// corridor steps deep. Branches elsewhere do not make the solution harder.
fn maze_decisions(grid: &[Vec<u8>], path: &[[usize; 2]]) -> (usize, usize) {
    let n = grid.len();
    let mut route = vec![vec![false; n]; n];
    for &[r, c] in path {
        route[r][c] = true;
    }
    let mut forks = 0;
    let mut decoys = 0;
    for &cell in path.iter().skip(1).take(path.len().saturating_sub(2)) {
        let branches: Vec<_> = neighbors(cell[0], cell[1], n, n)
            .into_iter()
            .filter(|p| grid[p[0]][p[1]] == 0 && !route[p[0]][p[1]])
            .collect();
        if !branches.is_empty() {
            forks += 1;
        }
        for branch in branches {
            let mut stack = vec![(branch, cell, 1)];
            let mut deepest = 0;
            while let Some((p, previous, depth)) = stack.pop() {
                deepest = deepest.max(depth);
                for next in neighbors(p[0], p[1], n, n) {
                    if next != previous && grid[next[0]][next[1]] == 0 && !route[next[0]][next[1]] {
                        stack.push((next, p, depth + 1));
                    }
                }
            }
            if deepest >= 4 {
                decoys += 1;
            }
        }
    }
    (forks, decoys)
}

// Retained so answers to existing v1 maze boards can still be verified.
fn maze_v1(rng: &mut ChaCha8Rng, difficulty: Difficulty) -> (Value, Value) {
    let cells = difficulty.select(5, 8, 12);
    let n = cells * 2 + 1;
    let mut grid = vec![vec![1u8; n]; n];
    grid[1][1] = 0;
    let mut stack = vec![(1usize, 1usize)];
    while let Some(&(r, c)) = stack.last() {
        let mut next = Vec::new();
        for (dr, dc) in [(-2isize, 0), (2, 0), (0, -2), (0, 2)] {
            let (rr, cc) = (r as isize + dr, c as isize + dc);
            if rr > 0
                && cc > 0
                && rr < (n - 1) as isize
                && cc < (n - 1) as isize
                && grid[rr as usize][cc as usize] == 1
            {
                next.push((rr as usize, cc as usize));
            }
        }
        if next.is_empty() {
            stack.pop();
        } else {
            let (rr, cc) = next[rng.random_range(0..next.len())];
            grid[(r + rr) / 2][(c + cc) / 2] = 0;
            grid[rr][cc] = 0;
            stack.push((rr, cc));
        }
    }
    let start = [1usize, 1usize];
    let end = [n - 2, n - 2];
    let mut parent = vec![vec![None; n]; n];
    parent[1][1] = Some(start);
    let mut queue = VecDeque::from([start]);
    while let Some([r, c]) = queue.pop_front() {
        if [r, c] == end {
            break;
        }
        for [rr, cc] in neighbors(r, c, n, n) {
            if grid[rr][cc] == 0 && parent[rr][cc].is_none() {
                parent[rr][cc] = Some([r, c]);
                queue.push_back([rr, cc]);
            }
        }
    }
    let mut path = vec![end];
    let mut current = end;
    while current != start {
        current = parent[current[0]][current[1]].expect("generated maze is connected");
        path.push(current);
    }
    path.reverse();
    (json!({"grid":grid,"start":start,"end":end}), json!(path))
}

fn clues(line: impl IntoIterator<Item = u8>) -> Vec<usize> {
    let mut result = Vec::new();
    let mut run = 0;
    for cell in line {
        if cell == 1 {
            run += 1;
        } else if run > 0 {
            result.push(run);
            run = 0;
        }
    }
    if run > 0 {
        result.push(run);
    }
    result
}

fn nonogram(rng: &mut ChaCha8Rng, difficulty: Difficulty) -> (Value, Value) {
    let n = difficulty.select(5usize, 7, 9);
    for _ in 0..16 {
        let grid: Vec<Vec<u8>> = (0..n)
            .map(|_| (0..n).map(|_| u8::from(rng.random_bool(0.55))).collect())
            .collect();
        let row_clues: Vec<_> = grid.iter().map(|row| clues(row.iter().copied())).collect();
        let column_clues: Vec<_> = (0..n).map(|c| clues((0..n).map(|r| grid[r][c]))).collect();
        let mut budget = 20_000;
        if nonogram_count(&row_clues, &column_clues, n, &mut budget) == 1 && budget > 0 {
            return (
                json!({"rows":n,"cols":n,"row_clues":row_clues,"column_clues":column_clues}),
                json!(grid),
            );
        }
    }
    // A bounded fallback: full/empty column clues force a unique grid.
    let mut pattern: Vec<u8> = (0..n).map(|_| u8::from(rng.random_bool(0.5))).collect();
    pattern[0] = 1;
    let grid = vec![pattern; n];
    let row_clues: Vec<_> = grid.iter().map(|row| clues(row.iter().copied())).collect();
    let column_clues: Vec<_> = (0..n).map(|c| clues((0..n).map(|r| grid[r][c]))).collect();
    (
        json!({"rows":n,"cols":n,"row_clues":row_clues,"column_clues":column_clues}),
        json!(grid),
    )
}

fn nonogram_count(rows: &[Vec<usize>], cols: &[Vec<usize>], n: usize, budget: &mut usize) -> usize {
    let patterns = |wanted: &Vec<usize>| {
        (0u16..1u16 << n)
            .filter(|bits| clues((0..n).map(|i| ((bits >> i) & 1) as u8)) == *wanted)
            .collect::<Vec<_>>()
    };
    let row_patterns: Vec<_> = rows.iter().map(patterns).collect();
    let col_patterns: Vec<_> = cols.iter().map(patterns).collect();
    fn search(
        row: usize,
        n: usize,
        rows: &[Vec<u16>],
        cols: &[Vec<u16>],
        prefix: &mut [u16],
        budget: &mut usize,
        limit: usize,
    ) -> usize {
        if *budget == 0 {
            return 0;
        }
        *budget -= 1;
        if row == n {
            return 1;
        }
        let mut count = 0;
        for pattern in &rows[row] {
            let mask = (1u16 << (row + 1)) - 1;
            for (c, col_prefix) in prefix.iter_mut().enumerate() {
                *col_prefix = (*col_prefix & !(1 << row)) | (((pattern >> c) & 1) << row);
            }
            if (0..n).all(|c| {
                cols[c]
                    .iter()
                    .any(|candidate| candidate & mask == prefix[c] & mask)
            }) {
                count += search(row + 1, n, rows, cols, prefix, budget, limit - count);
                if count >= limit || *budget == 0 {
                    break;
                }
            }
        }
        count
    }
    search(
        0,
        n,
        &row_patterns,
        &col_patterns,
        &mut vec![0; n],
        budget,
        2,
    )
}

const WORDS: &[&str] = &[
    "RUST", "CARGO", "PUZZLE", "LOGIC", "BINARY", "SERVER", "TOKEN", "MEMORY", "THREAD", "VECTOR",
    "BORROW", "TRAIT", "ASYNC", "FUTURE", "MODULE", "ENUM", "MATCH", "STACK", "QUEUE", "MUTEX",
    "STREAM", "SOCKET", "RANDOM", "SEED", "LIFETIME", "COMPILE", "ITERATOR", "LINUX", "WINDOWS",
    "TOWER", "AXUM", "RELEASE", "DEBUG", "STRING", "RESULT", "OPTION",
];

fn word_search(rng: &mut ChaCha8Rng, difficulty: Difficulty) -> (Value, Value) {
    let n = difficulty.select(10usize, 12, 15);
    let count = difficulty.select(5, 7, 10);
    let mut words = WORDS.to_vec();
    words.shuffle(rng);
    let mut grid = vec![vec!['\0'; n]; n];
    let mut placements = Vec::new();
    let mut selected = Vec::new();
    let directions = [
        (-1isize, -1isize),
        (-1, 0),
        (-1, 1),
        (0, -1),
        (0, 1),
        (1, -1),
        (1, 0),
        (1, 1),
    ];
    for word in words {
        if selected.len() == count {
            break;
        }
        let chars: Vec<char> = word.chars().collect();
        for _ in 0..128 {
            let (r, c) = (rng.random_range(0..n), rng.random_range(0..n));
            let (dr, dc) = directions[rng.random_range(0..directions.len())];
            let cells: Vec<_> = (0..chars.len())
                .map(|i| (r as isize + dr * i as isize, c as isize + dc * i as isize))
                .collect();
            if !cells.iter().zip(&chars).all(|(&(rr, cc), ch)| {
                rr >= 0
                    && cc >= 0
                    && rr < n as isize
                    && cc < n as isize
                    && (grid[rr as usize][cc as usize] == '\0'
                        || grid[rr as usize][cc as usize] == *ch)
            }) {
                continue;
            }
            let cells: Vec<[usize; 2]> = cells
                .into_iter()
                .map(|(rr, cc)| [rr as usize, cc as usize])
                .collect();
            for (&[rr, cc], ch) in cells.iter().zip(chars.iter()) {
                grid[rr][cc] = *ch;
            }
            selected.push(word);
            placements.push(json!({"word":word,"cells":cells}));
            break;
        }
    }
    for cell in grid.iter_mut().flatten() {
        if *cell == '\0' {
            *cell = (b'A' + rng.random_range(0..26)) as char;
        }
    }
    let grid: Vec<Vec<String>> = grid
        .into_iter()
        .map(|row| row.into_iter().map(|c| c.to_string()).collect())
        .collect();
    (json!({"grid":grid,"words":selected}), json!(placements))
}

fn verify_words(data: &Value, answer: Value) -> Result<bool, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Placement {
        word: String,
        cells: Vec<[usize; 2]>,
    }
    let submitted: Vec<Placement> = parse_answer(answer)?;
    let grid: Vec<Vec<String>> = parse_answer(data["grid"].clone())?;
    let mut words: Vec<String> = parse_answer(data["words"].clone())?;
    if submitted.len() != words.len() {
        return Ok(false);
    }
    let mut actual = Vec::new();
    for item in submitted {
        if item.cells.len() != item.word.len()
            || item.cells.len() < 2
            || item.cells.len() > grid.len()
        {
            return Ok(false);
        }
        let dr = item.cells[1][0] as i128 - item.cells[0][0] as i128;
        let dc = item.cells[1][1] as i128 - item.cells[0][1] as i128;
        if dr.abs() > 1 || dc.abs() > 1 || (dr == 0 && dc == 0) {
            return Ok(false);
        }
        for (i, ([r, c], ch)) in item.cells.iter().zip(item.word.chars()).enumerate() {
            if *r as i128 != item.cells[0][0] as i128 + dr * i as i128
                || *c as i128 != item.cells[0][1] as i128 + dc * i as i128
                || grid
                    .get(*r)
                    .and_then(|row| row.get(*c))
                    .is_none_or(|cell| cell != &ch.to_string())
            {
                return Ok(false);
            }
        }
        actual.push(item.word);
    }
    words.sort_unstable();
    actual.sort_unstable();
    Ok(words == actual)
}

fn solved_sliding(n: usize) -> Vec<Vec<u8>> {
    (0..n)
        .map(|r| (0..n).map(|c| ((r * n + c + 1) % (n * n)) as u8).collect())
        .collect()
}

fn slide(grid: &mut [Vec<u8>], direction: &str) -> bool {
    let n = grid.len();
    let Some((r, c)) = (0..n).find_map(|r| (0..n).find(|c| grid[r][*c] == 0).map(|c| (r, c)))
    else {
        return false;
    };
    let next = match direction {
        "up" if r > 0 => Some((r - 1, c)),
        "down" if r + 1 < n => Some((r + 1, c)),
        "left" if c > 0 => Some((r, c - 1)),
        "right" if c + 1 < n => Some((r, c + 1)),
        _ => None,
    };
    if let Some((rr, cc)) = next {
        grid[r][c] = grid[rr][cc];
        grid[rr][cc] = 0;
        true
    } else {
        false
    }
}

fn inverse(direction: &str) -> &'static str {
    match direction {
        "up" => "down",
        "down" => "up",
        "left" => "right",
        _ => "left",
    }
}

fn sliding(rng: &mut ChaCha8Rng, difficulty: Difficulty) -> (Value, Value) {
    let n = difficulty.select(3usize, 4, 4);
    let count = difficulty.select(24, 80, 160);
    let mut grid = solved_sliding(n);
    let mut history = Vec::<&str>::new();
    for _ in 0..count {
        let mut directions = ["up", "down", "left", "right"];
        directions.shuffle(rng);
        for direction in directions {
            if history
                .last()
                .is_some_and(|last| inverse(last) == direction)
            {
                continue;
            }
            if slide(&mut grid, direction) {
                history.push(direction);
                break;
            }
        }
    }
    if grid == solved_sliding(n) {
        slide(&mut grid, "up");
        history.push("up");
    }
    let moves: Vec<_> = history.into_iter().rev().map(inverse).collect();
    (json!({"grid":grid,"size":n}), json!({"moves":moves}))
}

fn neighbors(r: usize, c: usize, rows: usize, cols: usize) -> Vec<[usize; 2]> {
    let mut result = Vec::with_capacity(4);
    if r > 0 {
        result.push([r - 1, c]);
    }
    if r + 1 < rows {
        result.push([r + 1, c]);
    }
    if c > 0 {
        result.push([r, c - 1]);
    }
    if c + 1 < cols {
        result.push([r, c + 1]);
    }
    result
}

fn toggle(grid: &mut [Vec<u8>], r: usize, c: usize) {
    grid[r][c] ^= 1;
    for [rr, cc] in neighbors(r, c, grid.len(), grid[0].len()) {
        grid[rr][cc] ^= 1;
    }
}

fn lights(rng: &mut ChaCha8Rng, difficulty: Difficulty) -> (Value, Value) {
    let n = difficulty.select(4usize, 5, 6);
    let mut grid = vec![vec![0u8; n]; n];
    let mut presses = Vec::new();
    for r in 0..n {
        for c in 0..n {
            if rng.random_bool(0.4) {
                toggle(&mut grid, r, c);
                presses.push([r, c]);
            }
        }
    }
    if grid.iter().flatten().all(|cell| *cell == 0) {
        toggle(&mut grid, 0, 0);
        presses.push([0, 0]);
    }
    (json!({"grid":grid}), json!(presses))
}

fn surrounding(r: usize, c: usize, rows: usize, cols: usize) -> Vec<[usize; 2]> {
    let mut result = Vec::new();
    for rr in r.saturating_sub(1)..=(r + 1).min(rows - 1) {
        for cc in c.saturating_sub(1)..=(c + 1).min(cols - 1) {
            if [rr, cc] != [r, c] {
                result.push([rr, cc]);
            }
        }
    }
    result
}

fn minesweeper(rng: &mut ChaCha8Rng, difficulty: Difficulty) -> (Value, Value) {
    let n = difficulty.select(8usize, 12, 16);
    let mine_count = difficulty.select(10, 24, 50);
    let start = [n / 2, n / 2];
    let mut positions: Vec<_> = (0..n)
        .flat_map(|r| (0..n).map(move |c| [r, c]))
        .filter(|[r, c]| r.abs_diff(start[0]) > 1 || c.abs_diff(start[1]) > 1)
        .collect();
    positions.shuffle(rng);
    let mines = &positions[..mine_count];
    let mut mine_grid = vec![vec![false; n]; n];
    for &[r, c] in mines {
        mine_grid[r][c] = true;
    }
    let mut visited = vec![vec![false; n]; n];
    let mut queue = VecDeque::from([start]);
    visited[start[0]][start[1]] = true;
    let mut initial = Vec::new();
    while let Some([r, c]) = queue.pop_front() {
        let adjacent = surrounding(r, c, n, n)
            .iter()
            .filter(|[rr, cc]| mine_grid[*rr][*cc])
            .count();
        initial.push(json!({"row":r,"col":c,"adjacent":adjacent}));
        if adjacent == 0 {
            for [rr, cc] in surrounding(r, c, n, n) {
                if !visited[rr][cc] && !mine_grid[rr][cc] {
                    visited[rr][cc] = true;
                    queue.push_back([rr, cc]);
                }
            }
        }
    }
    (
        json!({"rows":n,"cols":n,"mine_count":mine_count,"start":start,"initial_revealed":initial}),
        json!(mines),
    )
}

fn arithmetic(rng: &mut ChaCha8Rng, difficulty: Difficulty) -> (Value, Value) {
    let max = difficulty.select(10i64, 30, 100);
    let a = rng.random_range(1..=max);
    let b = rng.random_range(1..=max);
    let c = rng.random_range(1..=max);
    let (expression, answer) = match difficulty {
        Difficulty::Easy => (format!("{a} + {b} - {c}"), a + b - c),
        Difficulty::Medium => (format!("({a} + {b}) * {c}"), (a + b) * c),
        Difficulty::Hard => {
            let d = rng.random_range(1..=max);
            (format!("({a} * {b}) - ({c} + {d})"), a * b - c - d)
        }
    };
    (json!({"expression":expression}), json!(answer))
}

fn cryptogram(rng: &mut ChaCha8Rng, difficulty: Difficulty) -> (Value, Value) {
    const SHORT: &[&str] = &[
        "THINK BEFORE YOU LEAP",
        "STAY CURIOUS",
        "PRACTICE MAKES PROGRESS",
        "FOLLOW THE CLUES",
        "SMALL STEPS ADD UP",
        "FIND YOUR OWN WAY",
        "KEEP AN OPEN MIND",
        "EVERY DETAIL COUNTS",
    ];
    const MEDIUM: &[&str] = &[
        "A JOURNEY OF A THOUSAND MILES BEGINS WITH A SINGLE STEP",
        "THE SECRET OF GETTING AHEAD IS GETTING STARTED",
        "GREAT THINGS ARE DONE BY A SERIES OF SMALL THINGS",
        "CURIOSITY IS THE WICK IN THE CANDLE OF LEARNING",
        "THE BEST WAY OUT IS ALWAYS THROUGH",
        "NOT ALL THOSE WHO WANDER ARE LOST",
        "WHAT WE KNOW IS A DROP WHAT WE DO NOT KNOW IS AN OCEAN",
        "SUCCESS IS THE SUM OF SMALL EFFORTS REPEATED",
    ];
    let phrases = difficulty.select(SHORT, MEDIUM, MEDIUM);
    let text = phrases[rng.random_range(0..phrases.len())];
    let mut alphabet = (b'A'..=b'Z').collect::<Vec<_>>();
    // A derangement avoids accidentally exposing letters. Bounded cyclic fallback.
    let mut valid = false;
    for _ in 0..128 {
        alphabet.shuffle(rng);
        if alphabet
            .iter()
            .enumerate()
            .all(|(i, c)| *c != b'A' + i as u8)
        {
            valid = true;
            break;
        }
    }
    if !valid {
        alphabet = (b'B'..=b'Z').chain(std::iter::once(b'A')).collect();
    }
    let ciphertext: String = text
        .bytes()
        .map(|ch| {
            if ch.is_ascii_uppercase() {
                alphabet[(ch - b'A') as usize] as char
            } else {
                ch as char
            }
        })
        .collect();
    (json!({"ciphertext":ciphertext}), json!(text))
}

#[cfg(test)]
mod tests {
    #[test]
    fn maze_routes_have_real_choices_and_deep_dead_ends() {
        use super::*;
        for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
            for seed in 0..128 {
                let puzzle = generate(request(PuzzleKind::Maze, seed, difficulty)).unwrap();
                let grid: Vec<Vec<u8>> =
                    serde_json::from_value(puzzle.data["grid"].clone()).unwrap();
                let path: Vec<[usize; 2]> =
                    serde_json::from_value(puzzle.solution.clone().unwrap()).unwrap();
                let (forks, decoys) = maze_decisions(&grid, &path);
                assert!(
                    forks >= difficulty.select(3, 5, 8),
                    "seed {seed} {difficulty:?}: only {forks} forks"
                );
                assert!(
                    decoys >= difficulty.select(2, 3, 5),
                    "seed {seed} {difficulty:?}: only {decoys} substantial wrong turns"
                );
                assert!(path.len() >= difficulty.select(21, 37, 57));
                let n = grid.len();
                let mut seen = vec![vec![false; n]; n];
                let mut stack = vec![[1, 1]];
                let mut count = 0;
                while let Some([r, c]) = stack.pop() {
                    if seen[r][c] {
                        continue;
                    }
                    seen[r][c] = true;
                    count += 1;
                    for p in neighbors(r, c, n, n) {
                        if grid[p[0]][p[1]] == 0 && !seen[p[0]][p[1]] {
                            stack.push(p);
                        }
                    }
                }
                let open = grid.iter().flatten().filter(|&&v| v == 0).count();
                let edges = (0..n)
                    .flat_map(|r| (0..n).map(move |c| (r, c)))
                    .filter(|&(r, c)| grid[r][c] == 0)
                    .map(|(r, c)| {
                        neighbors(r, c, n, n)
                            .into_iter()
                            .filter(|p| grid[p[0]][p[1]] == 0)
                            .count()
                    })
                    .sum::<usize>()
                    / 2;
                assert_eq!(count, open, "all paths must be reachable");
                assert_eq!(edges, open - 1, "exactly one simple route between cells");
                assert!(verify(request(PuzzleKind::Maze, seed, difficulty), json!(path)).unwrap());
            }
        }
    }
    #[test]
    fn existing_v1_maze_answers_remain_valid() {
        use super::*;
        for seed in 0..32 {
            let (_, answer) = maze_v1(&mut ChaCha8Rng::seed_from_u64(seed), Difficulty::Easy);
            assert!(verify(request(PuzzleKind::Maze, seed, Difficulty::Easy), answer).unwrap());
        }
    }
    use super::*;

    fn request(kind: PuzzleKind, seed: u64, difficulty: Difficulty) -> GenerateRequest {
        GenerateRequest {
            kind,
            seed: Some(seed),
            difficulty,
            include_solution: true,
            content_version: None,
        }
    }

    #[test]
    fn every_generator_is_deterministic_and_its_answer_verifies() {
        for kind in PuzzleKind::ALL {
            for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
                for seed in [0, 1, 42, u64::MAX] {
                    let req = request(kind, seed, difficulty);
                    let first = generate(req.clone()).unwrap();
                    assert_eq!(first, generate(req.clone()).unwrap(), "{}", kind.slug());
                    assert!(
                        verify(req, first.solution.unwrap()).unwrap(),
                        "{} seed {seed}",
                        kind.slug()
                    );
                }
            }
        }
    }

    #[test]
    fn solutions_are_opt_in_and_verification_requires_seed() {
        let mut req = request(PuzzleKind::Sudoku, 1, Difficulty::Medium);
        req.include_solution = false;
        assert!(generate(req.clone()).unwrap().solution.is_none());
        req.seed = None;
        assert!(verify(req, json!([])).is_err());
    }

    #[test]
    fn sudoku_clues_have_exactly_one_solution_and_respect_boxes() {
        for b in [2, 3] {
            for seed in 0..20 {
                let mut rng = ChaCha8Rng::seed_from_u64(seed);
                let (data, solution) = sudoku(&mut rng, b, Difficulty::Hard);
                let mut grid: Vec<Vec<u8>> = serde_json::from_value(data["grid"].clone()).unwrap();
                let solved: Vec<Vec<u8>> = serde_json::from_value(solution).unwrap();
                let n = b * b;
                for r in 0..n {
                    for c in 0..n {
                        assert!(grid[r][c] == 0 || grid[r][c] == solved[r][c]);
                    }
                }
                let expected: Vec<u8> = (1..=n as u8).collect();
                for original_row in &solved {
                    let mut row = original_row.clone();
                    row.sort_unstable();
                    assert_eq!(row, expected);
                }
                for (c, _) in solved[0].iter().enumerate() {
                    let mut col: Vec<_> = solved.iter().map(|row| row[c]).collect();
                    col.sort_unstable();
                    assert_eq!(col, expected);
                }
                for br in 0..b {
                    for bc in 0..b {
                        let mut block = Vec::new();
                        for row in solved.iter().skip(br * b).take(b) {
                            block.extend_from_slice(&row[bc * b..bc * b + b]);
                        }
                        block.sort_unstable();
                        assert_eq!(block, expected);
                    }
                }
                let mut budget = 500_000;
                assert_eq!(sudoku_count(&mut grid, b, 2, &mut budget), 1);
                assert!(budget > 0);
            }
        }
    }

    #[test]
    fn nonogram_clues_are_unique_and_match_solution() {
        for seed in 0..20 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let (data, solution) = nonogram(&mut rng, Difficulty::Hard);
            let grid: Vec<Vec<u8>> = serde_json::from_value(solution).unwrap();
            let n = grid.len();
            let rows: Vec<Vec<usize>> = serde_json::from_value(data["row_clues"].clone()).unwrap();
            let cols: Vec<Vec<usize>> =
                serde_json::from_value(data["column_clues"].clone()).unwrap();
            for r in 0..n {
                assert_eq!(rows[r], clues(grid[r].iter().copied()));
            }
            for c in 0..n {
                assert_eq!(cols[c], clues((0..n).map(|r| grid[r][c])));
            }
            let mut budget = 500_000;
            assert_eq!(nonogram_count(&rows, &cols, n, &mut budget), 1);
            assert!(budget > 0);
        }
    }

    #[test]
    fn alternate_legal_lights_patterns_and_maze_walks_are_accepted() {
        let req = request(PuzzleKind::LightsOut, 9, Difficulty::Medium);
        let puzzle = generate(req.clone()).unwrap();
        let mut solution: Vec<[usize; 2]> =
            serde_json::from_value(puzzle.solution.unwrap()).unwrap();
        solution.extend([[0, 0], [0, 0]]);
        assert!(verify(req, json!(solution)).unwrap());
        let req = request(PuzzleKind::Maze, 9, Difficulty::Medium);
        let puzzle = generate(req.clone()).unwrap();
        let mut solution: Vec<[usize; 2]> =
            serde_json::from_value(puzzle.solution.unwrap()).unwrap();
        let first = solution[0];
        let second = solution[1];
        solution.splice(1..1, [second, first]);
        assert!(verify(req, json!(solution)).unwrap());
    }

    #[test]
    fn malformed_and_impossible_answers_are_rejected() {
        assert!(
            !verify(
                request(PuzzleKind::Maze, 42, Difficulty::Easy),
                json!([[1, 1], [9, 9]])
            )
            .unwrap()
        );
        assert!(
            !verify(
                request(PuzzleKind::LightsOut, 42, Difficulty::Easy),
                json!([[100, 100]])
            )
            .unwrap()
        );
        assert!(
            !verify(
                request(PuzzleKind::SlidingPuzzle, 42, Difficulty::Easy),
                json!({"moves":["teleport"]})
            )
            .unwrap()
        );
        assert!(
            !verify(
                request(PuzzleKind::Minesweeper, 42, Difficulty::Easy),
                json!([[0, 0]])
            )
            .unwrap()
        );
        assert!(
            verify(
                request(PuzzleKind::Maze, 42, Difficulty::Easy),
                json!("oops")
            )
            .is_err()
        );
    }

    #[test]
    fn malformed_answers_never_panic_or_bypass_verification() {
        let malformed = [
            json!(null),
            json!(true),
            json!({}),
            json!(""),
            json!([-1]),
            json!([[u64::MAX, u64::MAX]]),
            json!({"moves": ["diagonal"]}),
        ];
        for kind in PuzzleKind::ALL {
            for answer in &malformed {
                assert!(
                    matches!(
                        verify(request(kind, u64::MAX, Difficulty::Hard), answer.clone()),
                        Ok(false) | Err(_)
                    ),
                    "malformed answer accepted for {}",
                    kind.slug()
                );
            }
        }
    }

    #[test]
    fn word_search_accepts_reordered_placements_and_rejects_duplicate_words() {
        let req = request(PuzzleKind::WordSearch, 17, Difficulty::Hard);
        let puzzle = generate(req.clone()).unwrap();
        let mut placements = puzzle.solution.unwrap().as_array().unwrap().clone();
        placements.reverse();
        assert!(verify(req.clone(), json!(placements)).unwrap());
        placements[0] = placements[1].clone();
        assert!(!verify(req, json!(placements)).unwrap());
    }

    #[test]
    fn mines_have_safe_zero_start_and_accurate_initial_counts() {
        for seed in 0..20 {
            let puzzle =
                generate(request(PuzzleKind::Minesweeper, seed, Difficulty::Hard)).unwrap();
            let mines: Vec<[usize; 2]> = serde_json::from_value(puzzle.solution.unwrap()).unwrap();
            let start: [usize; 2] = serde_json::from_value(puzzle.data["start"].clone()).unwrap();
            assert_eq!(mines.len(), 50);
            assert!(
                mines
                    .iter()
                    .all(|[r, c]| r.abs_diff(start[0]) > 1 || c.abs_diff(start[1]) > 1)
            );
            for cell in puzzle.data["initial_revealed"].as_array().unwrap() {
                let r = cell["row"].as_u64().unwrap() as usize;
                let c = cell["col"].as_u64().unwrap() as usize;
                assert!(!mines.contains(&[r, c]));
                let adjacent = surrounding(r, c, 16, 16)
                    .iter()
                    .filter(|cell| mines.contains(cell))
                    .count();
                assert_eq!(cell["adjacent"], json!(adjacent));
            }
        }
    }
}
