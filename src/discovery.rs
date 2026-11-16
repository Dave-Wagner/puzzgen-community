//! Seven bounded, seeded learning games. Original word selections and clues: CC0-1.0.
use crate::puzzles::{Difficulty, PuzzleKind};
use rand::{Rng, seq::SliceRandom};
use rand_chacha::ChaCha8Rng;
use serde_json::{Value, json};
use std::collections::{BTreeSet, VecDeque};

pub const KINDS: [PuzzleKind; 7] = [
    PuzzleKind::WordSorting,
    PuzzleKind::SequenceDetective,
    PuzzleKind::MemoryPairs,
    PuzzleKind::PathConnections,
    PuzzleKind::Futoshiki,
    PuzzleKind::SoundFamilies,
    PuzzleKind::SentenceAssembly,
];
fn level(d: Difficulty) -> usize {
    match d {
        Difficulty::Easy => 0,
        Difficulty::Medium => 1,
        Difficulty::Hard => 2,
    }
}
pub fn generate(kind: PuzzleKind, rng: &mut ChaCha8Rng, d: Difficulty) -> (Value, Value) {
    let l = level(d);
    let (mut data, solution, objective, principle) = match kind {
        PuzzleKind::WordSorting => {
            let mut groups = vec![
                (
                    "Musical instruments",
                    vec!["violin", "trumpet", "flute", "cello", "trombone"],
                ),
                (
                    "Geometric shapes",
                    vec!["triangle", "circle", "pentagon", "hexagon", "rectangle"],
                ),
                (
                    "Vehicles",
                    vec!["bicycle", "bus", "canoe", "tram", "scooter"],
                ),
                (
                    "Flowers",
                    vec!["daisy", "tulip", "daffodil", "marigold", "orchid"],
                ),
                (
                    "Clothing",
                    vec!["jacket", "scarf", "mitten", "sock", "apron"],
                ),
                (
                    "Kitchen utensils",
                    vec!["spatula", "ladle", "whisk", "colander", "grater"],
                ),
            ];
            groups.shuffle(rng);
            groups.truncate(l + 2);
            let mut items = vec![];
            let mut answer = serde_json::Map::new();
            let mut categories = vec![];
            for (i, (label, mut words)) in groups.into_iter().enumerate() {
                let category = format!("c{i}");
                categories.push(json!({"id":category,"label":label}));
                words.shuffle(rng);
                for text in words.into_iter().take(3) {
                    let id = format!("w{}", items.len());
                    answer.insert(id.clone(), json!(category));
                    items.push(json!({"id":id,"text":text}));
                }
            }
            items.shuffle(rng);
            (
                json!({"categories":categories,"items":items}),
                Value::Object(answer),
                "Classify using a stated rule",
                "A category is a rule for membership. Use the supplied everyday meanings; each item has one curated category.",
            )
        }
        PuzzleKind::SequenceDetective => {
            let family = rng.random_range(0..=l.min(2));
            let count = 6 + l;
            let start = rng.random_range(1_i64..=6);
            let step = rng.random_range(2_i64..=5);
            let terms: Vec<i64> = (0..count)
                .map(|i| match family {
                    0 => start + step * i as i64,
                    1 => start * step.pow(i as u32),
                    _ => (start + i as i64).pow(2),
                })
                .collect();
            let missing = rng.random_range(1..count - 1);
            let answer = terms[missing];
            let mut shown: Vec<_> = terms.into_iter().map(|n| json!(n)).collect();
            shown[missing] = Value::Null;
            (
                json!({"terms":shown,"missing_index":missing,"family":(["constant addition","constant multiplication","consecutive positive integer squares"][family])}),
                json!(answer),
                "Identify a rule and test it",
                "A finite sequence can fit many invented rules. Here the named family constrains the intended rule; test it against every visible term.",
            )
        }
        PuzzleKind::MemoryPairs => {
            let mut faces = vec![
                "acorn", "lantern", "flower", "robin", "shell", "star", "leaf", "moon", "pebble",
                "cloud", "feather", "sun",
            ];
            faces.shuffle(rng);
            faces.truncate(3 + l * 2);
            let mut deck = faces.iter().flat_map(|s| [*s, *s]).collect::<Vec<_>>();
            deck.shuffle(rng);
            let cards: Vec<_> = deck
                .iter()
                .enumerate()
                .map(|(i, s)| json!({"id":format!("m{i}"),"face":s}))
                .collect();
            let pairs: Vec<_> = faces
                .iter()
                .map(|face| {
                    deck.iter()
                        .enumerate()
                        .filter(|(_, s)| *s == face)
                        .map(|(i, _)| format!("m{i}"))
                        .collect::<Vec<_>>()
                })
                .collect();
            (
                json!({"cards":cards,"pair_count":faces.len()}),
                json!(pairs),
                "Remember positions and make associations",
                "Turn over two cards at a time and remember their positions. Faces are public construction data, not a secure hidden deck; your interface controls when they appear.",
            )
        }
        PuzzleKind::PathConnections => {
            let n = 3 + l;
            let mut grid = vec![vec![0_u8; n]; n];
            let mut seen = vec![vec![false; n]; n];
            seen[0][0] = true;
            fn carve(
                rng: &mut ChaCha8Rng,
                grid: &mut [Vec<u8>],
                seen: &mut [Vec<bool>],
                r: usize,
                c: usize,
            ) {
                let n = grid.len();
                let mut dirs = [0, 1, 2, 3];
                dirs.shuffle(rng);
                for dir in dirs {
                    if let Some((rr, cc)) = neighbor(n, r, c, dir)
                        && !seen[rr][cc]
                    {
                        seen[rr][cc] = true;
                        grid[r][c] |= 1 << dir;
                        grid[rr][cc] |= 1 << ((dir + 2) % 4);
                        carve(rng, grid, seen, rr, cc);
                    }
                }
            }
            carve(rng, &mut grid, &mut seen, 0, 0);
            let solution = json!(grid);
            for row in &mut grid {
                for mask in row {
                    for _ in 0..rng.random_range(0..4) {
                        *mask = rotate(*mask);
                    }
                }
            }
            (
                json!({"grid":grid,"size":n,"source":[0,0],"goal":[n-1,n-1],"direction_bits":{"north":1,"east":2,"south":4,"west":8}}),
                solution,
                "Connect a network without leaks",
                "Rotation preserves a tile's ports. Every port must meet its neighbor's opposite port, and every tile must connect to the source.",
            )
        }
        PuzzleKind::Futoshiki => {
            let mut symbols = [1_u8, 2, 3, 4];
            symbols.shuffle(rng);
            let mut rows = [0, 1, 2, 3];
            rows.shuffle(rng);
            let mut cols = [0, 1, 2, 3];
            cols.shuffle(rng);
            let solved: Vec<Vec<u8>> = rows
                .iter()
                .map(|r| cols.iter().map(|c| symbols[(r + c) % 4]).collect())
                .collect();
            let mut edges = vec![];
            for r in 0..4 {
                for c in 0..4 {
                    for (rr, cc) in [(r + 1, c), (r, c + 1)] {
                        if rr < 4 && cc < 4 {
                            edges.push(json!({"first":[r,c],"second":[rr,cc],"op":if solved[r][c]<solved[rr][cc]{"<"}else{">"}}));
                        }
                    }
                }
            }
            edges.shuffle(rng);
            edges.truncate(6 + l * 2);
            let mut grid = solved.clone();
            let mut cells = (0..16).collect::<Vec<_>>();
            cells.shuffle(rng);
            let mut remaining = 16;
            for cell in cells {
                if remaining <= 8 - l * 2 {
                    break;
                }
                let (r, c) = (cell / 4, cell % 4);
                let old = grid[r][c];
                grid[r][c] = 0;
                let mut budget = 20000;
                if latin_count(&mut grid, &edges, &mut budget) == 1 && budget > 0 {
                    remaining -= 1;
                } else {
                    grid[r][c] = old;
                }
            }
            (
                json!({"grid":grid,"size":4,"inequalities":edges,"clue_count":remaining}),
                json!(solved),
                "Combine Latin-square and ordering constraints",
                "Each row and column contains 1–4 once. Inequalities add ordering information. Clues are removed only when a bounded solver verifies a single completion.",
            )
        }
        PuzzleKind::SoundFamilies => {
            let mut families = vec![
                ("/aɪt/", vec!["light", "night", "kite", "white"]),
                ("/eɪn/", vec!["rain", "train", "lane", "chain"]),
                ("/æt/", vec!["cat", "hat", "mat", "bat"]),
                ("/oʊt/", vec!["boat", "coat", "goat", "note"]),
                ("/iː/", vec!["bee", "tree", "sea", "key"]),
                ("/uː/", vec!["blue", "shoe", "true", "new"]),
                ("/ɛl/", vec!["bell", "shell", "well", "tell"]),
                ("/ɔr/", vec!["door", "floor", "more", "shore"]),
            ];
            families.shuffle(rng);
            let (sound, mut matching) = families.remove(0);
            matching.shuffle(rng);
            let anchor = matching.remove(0);
            let mut words: Vec<_> = matching.into_iter().map(|w| (w, true)).collect();
            for (_, mut distractors) in families.into_iter().take(2 + l) {
                distractors.shuffle(rng);
                words.push((distractors[0], false));
            }
            words.shuffle(rng);
            let candidates: Vec<_> = words
                .iter()
                .enumerate()
                .map(|(i, (w, _))| json!({"id":format!("s{i}"),"text":w}))
                .collect();
            let answer: Vec<_> = words
                .iter()
                .enumerate()
                .filter(|(_, (_, yes))| *yes)
                .map(|(i, _)| format!("s{i}"))
                .collect();
            (
                json!({"anchor":anchor,"words":candidates,"convention":"general US English","hint":if l==0{sound}else{"Match the final stressed vowel and following sounds"}}),
                json!(answer),
                "Listen beyond spelling",
                "Rhyming words share a stressed vowel and the following sounds. Different spellings can represent the same ending; pronunciations vary by accent.",
            )
        }
        PuzzleKind::SentenceAssembly => {
            let nouns = [
                "rabbit", "fox", "robin", "turtle", "mouse", "cat", "otter", "badger",
            ];
            let adjectives = ["small", "curious", "sleepy", "brave", "gentle", "cheerful"];
            let verbs = ["rests", "waits", "wanders", "plays", "moves"];
            let adverbs = [
                "quietly",
                "happily",
                "gently",
                "calmly",
                "slowly",
                "patiently",
            ];
            let words = [
                "the",
                adjectives[rng.random_range(0..adjectives.len())],
                nouns[rng.random_range(0..nouns.len())],
                verbs[rng.random_range(0..verbs.len())],
                adverbs[rng.random_range(0..adverbs.len())],
            ];
            let mut tiles: Vec<_> = words
                .iter()
                .enumerate()
                .map(|(i, text)| json!({"id":format!("t{i}"),"text":text}))
                .collect();
            tiles.shuffle(rng);
            (
                json!({"tiles":tiles,"hint":if l==0{"Start with the, then describe the animal"}else{"The adverb can appear at the end, before the verb, or at the beginning"}}),
                json!(["t0", "t1", "t2", "t3", "t4"]),
                "Explore sentence structure and flexible word order",
                "A noun phrase identifies who acts, a verb describes the action, and an adverb describes how. This controlled grammar accepts three placements of the adverb, not every possible English sentence.",
            )
        }
        _ => unreachable!(),
    };
    data["learning"] = json!({"objective":objective,"principle":principle});
    data["content_license"] = json!("CC0-1.0");
    data["content_revision"] = json!("1");
    (data, solution)
}
pub fn rotate(mask: u8) -> u8 {
    ((mask << 1) & 15) | (mask >> 3)
}
fn neighbor(n: usize, r: usize, c: usize, dir: usize) -> Option<(usize, usize)> {
    match dir {
        0 if r > 0 => Some((r - 1, c)),
        1 if c + 1 < n => Some((r, c + 1)),
        2 if r + 1 < n => Some((r + 1, c)),
        3 if c > 0 => Some((r, c - 1)),
        _ => None,
    }
}
fn grid_answer(value: &Value, n: usize, max: u64) -> Option<Vec<Vec<u8>>> {
    let rows = value.as_array()?;
    if rows.len() != n {
        return None;
    }
    rows.iter()
        .map(|row| {
            let cells = row.as_array()?;
            if cells.len() != n {
                return None;
            }
            cells
                .iter()
                .map(|v| v.as_u64().filter(|v| *v <= max).map(|v| v as u8))
                .collect()
        })
        .collect()
}
fn strings(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(str::to_owned))
        .collect()
}
fn latin_ok(grid: &[Vec<u8>], edges: &[Value]) -> bool {
    for r in 0..4 {
        for c in 0..4 {
            let v = grid[r][c];
            if v > 4 {
                return false;
            }
            if v != 0 && (0..4).any(|i| (i != c && grid[r][i] == v) || (i != r && grid[i][c] == v))
            {
                return false;
            }
        }
    }
    edges.iter().all(|e| {
        let a = &e["first"];
        let b = &e["second"];
        let x = grid[a[0].as_u64().unwrap() as usize][a[1].as_u64().unwrap() as usize];
        let y = grid[b[0].as_u64().unwrap() as usize][b[1].as_u64().unwrap() as usize];
        x == 0 || y == 0 || if e["op"] == "<" { x < y } else { x > y }
    })
}
fn latin_count(grid: &mut [Vec<u8>], edges: &[Value], budget: &mut usize) -> usize {
    if *budget == 0 {
        return 2;
    }
    *budget -= 1;
    if !latin_ok(grid, edges) {
        return 0;
    }
    let Some((r, c)) = (0..4)
        .flat_map(|r| (0..4).map(move |c| (r, c)))
        .find(|(r, c)| grid[*r][*c] == 0)
    else {
        return 1;
    };
    let mut count = 0;
    for v in 1..=4 {
        grid[r][c] = v;
        count += latin_count(grid, edges, budget);
        if count >= 2 {
            break;
        }
    }
    grid[r][c] = 0;
    count.min(2)
}
pub fn verify(kind: PuzzleKind, data: &Value, solution: &Value, answer: &Value) -> bool {
    match kind {
        PuzzleKind::WordSorting | PuzzleKind::SequenceDetective => answer == solution,
        PuzzleKind::SentenceAssembly => {
            let Some(ids) = strings(answer) else {
                return false;
            };
            let canonical = strings(solution).unwrap();
            [
                vec![0, 1, 2, 3, 4],
                vec![0, 1, 2, 4, 3],
                vec![4, 0, 1, 2, 3],
            ]
            .iter()
            .any(|order| {
                ids == order
                    .iter()
                    .map(|i| canonical[*i].clone())
                    .collect::<Vec<_>>()
            })
        }
        PuzzleKind::SoundFamilies => {
            let Some(ids) = strings(answer) else {
                return false;
            };
            let expected = strings(solution).unwrap();
            ids.len() == expected.len()
                && ids.into_iter().collect::<BTreeSet<_>>() == expected.into_iter().collect()
        }
        PuzzleKind::MemoryPairs => {
            let Some(pairs) = answer.as_array() else {
                return false;
            };
            let cards = data["cards"].as_array().unwrap();
            if pairs.len() * 2 != cards.len() {
                return false;
            }
            let mut used = BTreeSet::new();
            pairs.iter().all(|pair| {
                let Some(ids) = strings(pair) else {
                    return false;
                };
                if ids.len() != 2 {
                    return false;
                }
                let mut faces = vec![];
                for id in ids {
                    if !used.insert(id.clone()) {
                        return false;
                    }
                    let Some(card) = cards.iter().find(|c| c["id"].as_str() == Some(&id)) else {
                        return false;
                    };
                    faces.push(&card["face"]);
                }
                faces[0] == faces[1]
            })
        }
        PuzzleKind::Futoshiki => {
            let Some(grid) = grid_answer(answer, 4, 4) else {
                return false;
            };
            let givens = grid_answer(&data["grid"], 4, 4).unwrap();
            grid.iter().flatten().all(|v| *v > 0)
                && (0..4).all(|r| (0..4).all(|c| givens[r][c] == 0 || givens[r][c] == grid[r][c]))
                && latin_ok(&grid, data["inequalities"].as_array().unwrap())
        }
        PuzzleKind::PathConnections => {
            let n = data["size"].as_u64().unwrap() as usize;
            let Some(grid) = grid_answer(answer, n, 15) else {
                return false;
            };
            let original = grid_answer(&data["grid"], n, 15).unwrap();
            for r in 0..n {
                for c in 0..n {
                    let mut mask = original[r][c];
                    if !(0..4).any(|_| {
                        let same = mask == grid[r][c];
                        mask = rotate(mask);
                        same
                    }) {
                        return false;
                    }
                    for dir in 0..4 {
                        if grid[r][c] & (1 << dir) != 0 {
                            let Some((rr, cc)) = neighbor(n, r, c, dir) else {
                                return false;
                            };
                            if grid[rr][cc] & (1 << ((dir + 2) % 4)) == 0 {
                                return false;
                            }
                        }
                    }
                }
            }
            let mut visited = BTreeSet::from([(0, 0)]);
            let mut queue = VecDeque::from([(0, 0)]);
            while let Some((r, c)) = queue.pop_front() {
                for dir in 0..4 {
                    if grid[r][c] & (1 << dir) != 0 {
                        let (rr, cc) = neighbor(n, r, c, dir).unwrap();
                        if visited.insert((rr, cc)) {
                            queue.push_back((rr, cc));
                        }
                    }
                }
            }
            visited.len() == n * n
        }
        _ => false,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    #[test]
    fn seeded_answers_are_valid_and_malformed_answers_fail() {
        for kind in KINDS {
            for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
                for seed in 0..30 {
                    let (d, s) = generate(kind, &mut ChaCha8Rng::seed_from_u64(seed), difficulty);
                    assert_eq!(
                        (d.clone(), s.clone()),
                        generate(kind, &mut ChaCha8Rng::seed_from_u64(seed), difficulty)
                    );
                    assert!(verify(kind, &d, &s, &s), "{kind:?} {seed}");
                    for bad in [Value::Null, json!({}), json!([]), json!("wrong"), json!(-1)] {
                        assert!(!verify(kind, &d, &s, &bad), "{kind:?}");
                    }
                }
            }
        }
    }
    #[test]
    fn futoshiki_really_has_one_completion() {
        for seed in 0..50 {
            let (d, _) = generate(
                PuzzleKind::Futoshiki,
                &mut ChaCha8Rng::seed_from_u64(seed),
                Difficulty::Hard,
            );
            let mut grid = grid_answer(&d["grid"], 4, 4).unwrap();
            let mut budget = 100000;
            assert_eq!(
                latin_count(
                    &mut grid,
                    d["inequalities"].as_array().unwrap(),
                    &mut budget
                ),
                1
            );
            assert!(budget > 0);
        }
    }
    #[test]
    fn alternative_answers_and_illegal_moves() {
        let (d, s) = generate(
            PuzzleKind::SentenceAssembly,
            &mut ChaCha8Rng::seed_from_u64(3),
            Difficulty::Easy,
        );
        assert!(verify(
            PuzzleKind::SentenceAssembly,
            &d,
            &s,
            &json!(["t4", "t0", "t1", "t2", "t3"])
        ));
        assert!(!verify(
            PuzzleKind::SentenceAssembly,
            &d,
            &s,
            &json!(["t0", "t1", "t1", "t3", "t4"])
        ));
        let (d, s) = generate(
            PuzzleKind::PathConnections,
            &mut ChaCha8Rng::seed_from_u64(3),
            Difficulty::Easy,
        );
        let mut bad = s.clone();
        bad[0][0] = json!(0);
        assert!(!verify(PuzzleKind::PathConnections, &d, &s, &bad));
        let (d, s) = generate(
            PuzzleKind::SoundFamilies,
            &mut ChaCha8Rng::seed_from_u64(3),
            Difficulty::Easy,
        );
        let mut ids = s.as_array().unwrap().clone();
        ids.reverse();
        assert!(verify(PuzzleKind::SoundFamilies, &d, &s, &json!(ids)));
        ids[0] = ids[1].clone();
        assert!(!verify(PuzzleKind::SoundFamilies, &d, &s, &json!(ids)));
    }
}
