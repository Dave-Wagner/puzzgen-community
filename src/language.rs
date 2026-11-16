//! Bounded, offline language puzzles backed by original CC0 content.
use crate::puzzles::{Difficulty, PuzzleKind};
use rand::{Rng, seq::SliceRandom};
use rand_chacha::ChaCha8Rng;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, VecDeque},
    sync::OnceLock,
};

#[derive(Deserialize)]
struct Spoon {
    phrases: [String; 2],
    clues: [String; 2],
    onsets: [String; 2],
    rimes: [String; 2],
}
#[derive(Deserialize)]
struct Corpus {
    version: String,
    license: String,
    locale: String,
    spoonerisms: Vec<Spoon>,
    anagrams: Vec<[String; 2]>,
    bridges: Vec<[String; 4]>,
    ladders: Vec<Vec<String>>,
}
fn corpus_version(version: &str) -> &'static Corpus {
    static OLD: OnceLock<Corpus> = OnceLock::new();
    static CORPUS: OnceLock<Corpus> = OnceLock::new();
    let (slot, text) = if version == "1" {
        (&OLD, include_str!("../data/language/en-us-v1.json"))
    } else {
        (&CORPUS, include_str!("../data/language/en-us-v2.json"))
    };
    slot.get_or_init(|| {
        serde_json::from_str(text.trim_start_matches('\u{feff}')).expect("embedded language corpus")
    })
}
pub fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
fn level(d: Difficulty) -> usize {
    match d {
        Difficulty::Easy => 0,
        Difficulty::Medium => 1,
        Difficulty::Hard => 2,
    }
}
fn letters(s: &str) -> Vec<char> {
    let mut v: Vec<_> = s.chars().collect();
    v.sort_unstable();
    v
}
fn adjacent(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).filter(|(a, b)| a != b).count() == 1
}
fn vocabulary(c: &Corpus, len: usize) -> Vec<String> {
    c.ladders
        .iter()
        .flatten()
        .filter(|w| w.len() == len)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn shortest(words: &[String], start: &str, end: &str) -> Vec<String> {
    let mut queue = VecDeque::from([vec![start.to_owned()]]);
    let mut seen = BTreeSet::from([start.to_owned()]);
    while let Some(path) = queue.pop_front() {
        let last = path.last().unwrap();
        if last == end {
            return path;
        }
        for next in words {
            if adjacent(last, next) && seen.insert(next.clone()) {
                let mut p = path.clone();
                p.push(next.clone());
                queue.push_back(p);
            }
        }
    }
    panic!("curated ladder must be connected")
}
pub fn generate(kind: PuzzleKind, rng: &mut ChaCha8Rng, d: Difficulty) -> (Value, Value) {
    generate_version(kind, rng, d, "2")
}
pub fn generate_version(
    kind: PuzzleKind,
    rng: &mut ChaCha8Rng,
    d: Difficulty,
    version: &str,
) -> (Value, Value) {
    let c = corpus_version(version);
    let l = level(d);
    let (mut data, answer) = match kind {
        PuzzleKind::Spoonerism => {
            let e = &c.spoonerisms[rng.random_range(0..c.spoonerisms.len())];
            let direction = rng.random_range(0..2);
            let target = 1 - direction;
            let mut v = json!({"source_phrase":e.phrases[direction],"clue":e.clues[target],"word_lengths":e.phrases[target].split_whitespace().map(str::len).collect::<Vec<_>>()});
            if l == 0 {
                let mut choices = c
                    .spoonerisms
                    .iter()
                    .flat_map(|e| e.phrases.iter())
                    .filter(|p| **p != e.phrases[target] && **p != e.phrases[direction])
                    .cloned()
                    .collect::<Vec<_>>();
                if version != "1" {
                    choices.shuffle(rng);
                }
                choices.truncate(3);
                choices.push(e.phrases[target].clone());
                choices.shuffle(rng);
                v["choices"] = json!(choices);
            }
            if l < 2 {
                v["hint"] = json!(
                    "Swap the initial consonant sounds of the two words, keeping the remaining sounds in order."
                );
                v["source_sounds"] = json!([
                    format!("{}{}", e.onsets[direction], e.rimes[0]),
                    format!("{}{}", e.onsets[1 - direction], e.rimes[1])
                ]);
            }
            (v, json!(e.phrases[target]))
        }
        PuzzleKind::Anagram => {
            let e = &c.anagrams[rng.random_range(0..c.anagrams.len())];
            let mut chars: Vec<_> = e[0].chars().collect();
            chars.shuffle(rng);
            if chars.iter().collect::<String>() == e[0] {
                chars.rotate_left(1);
            }
            let mut v = json!({"letters":chars.iter().collect::<String>(),"clue":e[1],"length":chars.len()});
            if l == 0 {
                v["first_letter"] = json!(&e[0][..1]);
            }
            if l == 2 {
                v.as_object_mut().unwrap().remove("clue");
            }
            // Hard mode has no semantic clue, so every listed anagram is accepted.
            (v, json!(e[0]))
        }
        PuzzleKind::CompoundBridge => {
            let e = &c.bridges[rng.random_range(0..c.bridges.len())];
            let mut v = json!({"left":e[0],"right":e[2],"length":e[1].len()});
            if l < 2 {
                v["clue"] = json!(e[3]);
            }
            if l == 0 {
                v["first_letter"] = json!(&e[1][..1]);
            }
            (v, json!(e[1]))
        }
        PuzzleKind::WordLadder => {
            let chain = &c.ladders[rng.random_range(0..c.ladders.len())];
            let size = (l + 3).min(chain.len());
            let start = rng.random_range(0..=chain.len() - size);
            let (mut a, mut b) = (chain[start].clone(), chain[start + size - 1].clone());
            if rng.random_bool(0.5) {
                std::mem::swap(&mut a, &mut b);
            }
            let words = if version == "1" {
                vocabulary(c, a.len())
            } else {
                // A small visible dictionary makes the graph approachable; the
                // source chain guarantees connectivity even with distractors.
                let peers: Vec<_> = c.ladders.iter().filter(|p| p[0].len() == a.len()).collect();
                let other = peers[rng.random_range(0..peers.len())];
                chain
                    .iter()
                    .chain(other.iter())
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            };
            let path = shortest(&words, &a, &b);
            (
                json!({"start":a,"end":b,"allowed_words":words,"max_steps":path.len()-1+ l,"minimum_steps":path.len()-1}),
                json!(path),
            )
        }
        _ => unreachable!("not a language kind"),
    };
    data["content_version"] = json!(c.version);
    data["locale"] = json!(c.locale);
    data["content_license"] = json!(c.license);
    if version != "1" {
        let (objective, principle, experiment) = match kind {
            PuzzleKind::Spoonerism => (
                "Hear how words are built from sounds",
                "An onset is the opening consonant sound or cluster. Swap the onsets, keeping each word's remaining sounds in place. Spelling may change.",
                "Say the starting phrase slowly, then exchange only its opening sounds.",
            ),
            PuzzleKind::Anagram => (
                "Recognize a letter multiset",
                "Anagrams preserve the number of each letter, including duplicates. A clue can distinguish words that use exactly the same letters.",
                "Sort the supplied letters, or count how many times each letter appears.",
            ),
            PuzzleKind::WordLadder => (
                "Explore a graph one edge at a time",
                "Words are vertices; a one-letter substitution creates an edge. Breadth-first search finds a shortest route, but other legal routes can also work.",
                "Compare two neighboring words: exactly one position should change.",
            ),
            _ => (
                "Connect words through meaning",
                "A bridge word participates in two established compounds. Meaning and usage matter; joining arbitrary words does not guarantee a familiar compound.",
                "Read the left word with your answer, then your answer with the right word.",
            ),
        };
        data["learning"] =
            json!({"objective":objective,"principle":principle,"experiment":experiment});
    }
    // Hard anagrams need their entire accepted vocabulary exposed to avoid hidden rules.
    if kind == PuzzleKind::Anagram && l == 2 {
        data["allowed_words"] = json!(c.anagrams.iter().map(|e| &e[0]).collect::<Vec<_>>());
    }
    (data, answer)
}
pub fn verify(kind: PuzzleKind, data: &Value, solution: &Value, answer: &Value) -> bool {
    if kind == PuzzleKind::WordLadder {
        let Some(path) = answer.as_array() else {
            return false;
        };
        let max = data["max_steps"].as_u64().unwrap() as usize;
        if path.len() < 2 || path.len() > max + 1 {
            return false;
        }
        let Some(words) = path
            .iter()
            .map(|v| v.as_str().map(normalize))
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        let allowed = data["allowed_words"].as_array().unwrap();
        return words.first().unwrap() == data["start"].as_str().unwrap()
            && words.last().unwrap() == data["end"].as_str().unwrap()
            && words
                .iter()
                .all(|w| allowed.iter().any(|v| v.as_str() == Some(w.as_str())))
            && words.iter().collect::<BTreeSet<_>>().len() == words.len()
            && words.windows(2).all(|p| adjacent(&p[0], &p[1]));
    }
    let Some(text) = answer.as_str() else {
        return false;
    };
    let text = normalize(text);
    if kind == PuzzleKind::Anagram
        && let Some(allowed) = data.get("allowed_words").and_then(Value::as_array)
    {
        return allowed.iter().any(|v| v.as_str() == Some(text.as_str()))
            && letters(&text) == letters(solution.as_str().unwrap());
    }
    text == normalize(solution.as_str().unwrap())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corpus_integrity() {
        for version in ["1", "2"] {
            let c = corpus_version(version);
            assert_eq!(c.license, "CC0-1.0");
            assert_eq!(c.version, version);
            assert_eq!(
                c.anagrams
                    .iter()
                    .map(|e| &e[0])
                    .collect::<BTreeSet<_>>()
                    .len(),
                c.anagrams.len()
            );
            assert_eq!(
                c.bridges
                    .iter()
                    .map(|e| (&e[0], &e[2]))
                    .collect::<BTreeSet<_>>()
                    .len(),
                c.bridges.len()
            );
            for e in &c.spoonerisms {
                assert_ne!(e.onsets[0], e.onsets[1]);
                assert!(e.rimes.iter().all(|s| !s.is_empty()));
                assert!(e.phrases.iter().all(|p| p.split_whitespace().count() == 2));
                assert!(e.clues.iter().all(|s| !s.is_empty()));
            }
            for e in &c.anagrams {
                assert!(e[0].bytes().all(|b| b.is_ascii_lowercase()));
                assert!(e[0].len() > 2);
            }
            for p in &c.ladders {
                assert!(p.windows(2).all(|p| adjacent(&p[0], &p[1])));
                assert_eq!(p.iter().collect::<BTreeSet<_>>().len(), p.len());
            }
            for e in &c.bridges {
                assert!(e.iter().all(|s| !s.is_empty()));
            }
        }
    }
    #[test]
    fn expanded_content_has_measurable_seed_diversity() {
        use rand::SeedableRng;
        for (kind, minimum) in [
            (PuzzleKind::Spoonerism, 20),
            (PuzzleKind::Anagram, 120),
            (PuzzleKind::CompoundBridge, 60),
            (PuzzleKind::WordLadder, 100),
        ] {
            let mut distinct = BTreeSet::new();
            for seed in 0..1000 {
                let (data, answer) = generate(
                    kind,
                    &mut ChaCha8Rng::seed_from_u64(seed),
                    Difficulty::Medium,
                );
                let identity = match kind {
                    PuzzleKind::Spoonerism => data["source_phrase"].to_string(),
                    PuzzleKind::Anagram => answer.to_string(),
                    PuzzleKind::CompoundBridge => format!("{}:{}", data["left"], data["right"]),
                    _ => format!("{}:{}", data["start"], data["end"]),
                };
                distinct.insert(identity);
                assert!(verify(kind, &data, &answer, &answer));
                if kind == PuzzleKind::WordLadder {
                    assert!(data["allowed_words"].as_array().unwrap().len() <= 16);
                    assert_eq!(
                        answer.as_array().unwrap().len() - 1,
                        data["minimum_steps"].as_u64().unwrap() as usize
                    );
                }
            }
            assert!(
                distinct.len() >= minimum,
                "{kind:?}: only {} distinct challenges",
                distinct.len()
            );
        }
    }
    #[test]
    fn rejects_wrong_answer_shapes_and_answers() {
        use rand::SeedableRng;
        for kind in [
            PuzzleKind::Spoonerism,
            PuzzleKind::Anagram,
            PuzzleKind::CompoundBridge,
            PuzzleKind::WordLadder,
        ] {
            for d in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
                for seed in 0..100 {
                    let (data, solution) = generate(kind, &mut ChaCha8Rng::seed_from_u64(seed), d);
                    assert!(verify(kind, &data, &solution, &solution));
                    assert!(!verify(kind, &data, &solution, &json!(null)));
                    assert!(!verify(kind, &data, &solution, &json!("not an answer")));
                    assert!(!verify(kind, &data, &solution, &json!([])));
                }
            }
        }
    }
    #[test]
    fn alternative_ladders_and_anagrams_are_accepted() {
        let data = json!({"start":"cat","end":"dog","max_steps":3,"allowed_words":["cat","cot","cog","dog","dat","dot"]});
        assert!(verify(
            PuzzleKind::WordLadder,
            &data,
            &json!(["cat", "cot", "cog", "dog"]),
            &json!(["CAT", "dat", "dot", "dog"])
        ));
        assert!(!verify(
            PuzzleKind::WordLadder,
            &data,
            &Value::Null,
            &json!(["cat", "dog"])
        ));
        assert!(!verify(
            PuzzleKind::WordLadder,
            &data,
            &Value::Null,
            &json!(["cat", "cot", "cat", "dog"])
        ));
        assert!(verify(
            PuzzleKind::Anagram,
            &json!({"allowed_words":["listen","silent"]}),
            &json!("listen"),
            &json!(" SILENT ")
        ));
    }
}
