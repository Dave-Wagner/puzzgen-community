use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const KINDS: [&str; 10] = [
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
];
pub const TITLES: [&str; 10] = [
    "Emerald Library",
    "Toybox Garden",
    "The Lost Lantern",
    "Pixel Studio",
    "Woodland Expedition",
    "Sunken Temple",
    "Orbital Relay",
    "Salvage Depot",
    "Clockwork Workshop",
    "Arcane Archive",
];
pub const NAMES: [&str; 10] = [
    "Sudoku",
    "Mini Sudoku",
    "Maze",
    "Nonogram",
    "Word Search",
    "Sliding Tiles",
    "Lights Out",
    "Minesweeper",
    "Arithmetic",
    "Cryptogram",
];
pub const ICONS: [&str; 10] = ["▦", "✿", "♜", "▧", "❧", "◈", "⏣", "⚑", "⚙", "✧"];
pub const HELP: [&str; 10] = [
    "Tap a blank square, then a number. Each row, column and 3×3 box must contain 1–9 once. Tap Erase to clear a mistake.",
    "Fill each row, column and 2×2 box with 1–4, once each. A small board for a big little victory.",
    "Guide the lantern to the exit. Tap an adjacent corridor or use the arrows. You can retrace your steps.",
    "Numbers describe consecutive runs of painted squares, in order. Leave a gap between runs. Switch to Cross to mark empty squares.",
    "Tap the first and last letter of a word in a straight line. Words may run in any of eight directions, including backwards.",
    "Tap a tile beside the empty space to slide it. Arrange numbers in reading order with the empty space last.",
    "Turn every light off. Tapping a light flips it and its four neighbours. Tapping twice cancels that switch.",
    "Reveal safe squares; numbers count neighbouring mines. Switch to Flag to mark all mines. Zeroes open surrounding safe squares. Some boards require a guess.",
    "Solve the expression. Parentheses first, then multiplication, then addition and subtraction. Use the keypad to enter your answer.",
    "Each cipher letter stands for one plain letter. Tap a cipher letter, then assign a letter below. A plain letter can be used only once.",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Puzzle {
    pub kind: String,
    pub seed: String,
    pub difficulty: String,
    pub data: Value,
    #[serde(default)]
    pub solution: Option<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub puzzle: Puzzle,
    pub live: bool,
    pub grid: Vec<Vec<i32>>,
    pub selected: Option<[usize; 2]>,
    pub path: Vec<[usize; 2]>,
    pub found: Vec<Value>,
    pub moves: Vec<String>,
    pub presses: Vec<[usize; 2]>,
    pub flags: Vec<[usize; 2]>,
    pub revealed: Vec<[usize; 2]>,
    pub mapping: Vec<(char, char)>,
    pub text: String,
    pub alternate: bool,
    pub actions: u32,
    pub assisted: bool,
    pub won: bool,
    pub lost: bool,
}
pub fn numeric_grid(value: &Value) -> Vec<Vec<i32>> {
    value
        .as_array()
        .map(|a| {
            a.iter()
                .map(|r| {
                    r.as_array()
                        .map(|r| r.iter().map(|n| n.as_i64().unwrap_or(0) as i32).collect())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}
pub fn coords(value: &Value) -> Vec<[usize; 2]> {
    serde_json::from_value(value.clone()).unwrap_or_default()
}
impl Game {
    /// Completion follows puzzle rules, without a developer-facing Check action
    /// or a downloaded answer key. Given Sudoku cells must remain unchanged.
    pub fn is_solved(&self) -> bool {
        if self.lost {
            return false;
        }
        match self.puzzle.kind.as_str() {
            "sudoku" | "mini-sudoku" => {
                let n = if self.puzzle.kind == "sudoku" { 9 } else { 4 };
                let box_size = if n == 9 { 3 } else { 2 };
                if self.grid.len() != n || self.grid.iter().any(|r| r.len() != n) {
                    return false;
                }
                let complete = |values: Vec<i32>| {
                    let mut values = values;
                    values.sort_unstable();
                    values == (1..=n as i32).collect::<Vec<_>>()
                };
                for r in 0..n {
                    if !complete(self.grid[r].clone())
                        || !complete((0..n).map(|c| self.grid[c][r]).collect())
                    {
                        return false;
                    }
                    for c in 0..n {
                        let given = self.puzzle.data["grid"][r][c].as_i64().unwrap_or(0) as i32;
                        if given != 0 && self.grid[r][c] != given {
                            return false;
                        }
                    }
                }
                for br in (0..n).step_by(box_size) {
                    for bc in (0..n).step_by(box_size) {
                        if !complete(
                            (br..br + box_size)
                                .flat_map(|r| (bc..bc + box_size).map(move |c| self.grid[r][c]))
                                .collect(),
                        ) {
                            return false;
                        }
                    }
                }
                true
            }
            "word-search" | "maze" => self.practice_correct(),
            _ => false,
        }
    }
    pub fn new(puzzle: Puzzle, live: bool) -> Self {
        let mut grid = numeric_grid(&puzzle.data["grid"]);
        if grid.is_empty() {
            grid = vec![
                vec![0; puzzle.data["cols"].as_u64().unwrap_or(1) as usize];
                puzzle.data["rows"].as_u64().unwrap_or(1) as usize
            ];
        }
        let path = if puzzle.kind == "maze" {
            vec![serde_json::from_value(puzzle.data["start"].clone()).unwrap_or([1, 1])]
        } else {
            vec![]
        };
        let revealed = puzzle.data["initial_revealed"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|c| {
                        [
                            c["row"].as_u64().unwrap_or(0) as usize,
                            c["col"].as_u64().unwrap_or(0) as usize,
                        ]
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            puzzle,
            live,
            grid,
            selected: None,
            path,
            found: vec![],
            moves: vec![],
            presses: vec![],
            flags: vec![],
            revealed,
            mapping: vec![],
            text: String::new(),
            alternate: false,
            actions: 0,
            assisted: false,
            won: false,
            lost: false,
        }
    }
    pub fn index(&self) -> usize {
        KINDS
            .iter()
            .position(|k| *k == self.puzzle.kind)
            .unwrap_or(0)
    }
    pub fn rows(&self) -> usize {
        self.grid.len()
    }
    pub fn cols(&self) -> usize {
        self.grid.first().map_or(0, Vec::len)
    }
    pub fn mine(&self, p: [usize; 2]) -> bool {
        self.puzzle
            .solution
            .as_ref()
            .is_some_and(|s| coords(s).contains(&p))
    }
    pub fn adjacent(&self, p: [usize; 2]) -> usize {
        (0..self.rows())
            .flat_map(|r| (0..self.cols()).map(move |c| [r, c]))
            .filter(|q| {
                q != &p && q[0].abs_diff(p[0]) <= 1 && q[1].abs_diff(p[1]) <= 1 && self.mine(*q)
            })
            .count()
    }
    pub fn tap(&mut self, p: [usize; 2]) {
        if self.won || self.lost || p[0] >= self.rows() || p[1] >= self.cols() {
            return;
        }
        match self.puzzle.kind.as_str() {
            "sudoku" | "mini-sudoku" => {
                self.selected = Some(p);
            }
            "maze" => {
                let Some(current) = self.path.last().copied() else {
                    return;
                };
                if self.grid[p[0]][p[1]] == 0
                    && current[0].abs_diff(p[0]) + current[1].abs_diff(p[1]) == 1
                {
                    if let Some(i) = self.path.iter().position(|q| q == &p) {
                        self.path.truncate(i + 1);
                    } else {
                        self.path.push(p);
                    }
                    self.actions += 1;
                }
            }
            "nonogram" => {
                let v = if self.alternate { -1 } else { 1 };
                self.grid[p[0]][p[1]] = if self.grid[p[0]][p[1]] == v { 0 } else { v };
                self.actions += 1;
            }
            "word-search" => {
                if let Some(start) = self.selected.take() {
                    self.find_word(start, p);
                } else {
                    self.selected = Some(p);
                }
            }
            "sliding-puzzle" => {
                let empty = (0..self.rows())
                    .flat_map(|r| (0..self.cols()).map(move |c| [r, c]))
                    .find(|q| self.grid[q[0]][q[1]] == 0);
                if let Some(e) = empty
                    && e[0].abs_diff(p[0]) + e[1].abs_diff(p[1]) == 1
                {
                    self.moves.push(
                        if p[0] < e[0] {
                            "up"
                        } else if p[0] > e[0] {
                            "down"
                        } else if p[1] < e[1] {
                            "left"
                        } else {
                            "right"
                        }
                        .into(),
                    );
                    self.grid[e[0]][e[1]] = self.grid[p[0]][p[1]];
                    self.grid[p[0]][p[1]] = 0;
                    self.actions += 1;
                }
            }
            "lights-out" => {
                for (dr, dc) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let r = p[0] as isize + dr;
                    let c = p[1] as isize + dc;
                    if r >= 0 && c >= 0 && (r as usize) < self.rows() && (c as usize) < self.cols()
                    {
                        self.grid[r as usize][c as usize] ^= 1;
                    }
                }
                toggle(&mut self.presses, p);
                self.actions += 1;
            }
            "minesweeper" => {
                if self.revealed.contains(&p) {
                    return;
                }
                if self.alternate {
                    toggle(&mut self.flags, p);
                } else if !self.flags.contains(&p) {
                    if self.mine(p) {
                        self.lost = true;
                        self.revealed.push(p);
                    } else {
                        self.reveal(p);
                    }
                }
                self.actions += 1;
            }
            _ => {}
        }
    }
    fn reveal(&mut self, p: [usize; 2]) {
        let mut queue = vec![p];
        while let Some(q) = queue.pop() {
            if self.revealed.contains(&q) || self.flags.contains(&q) || self.mine(q) {
                continue;
            }
            self.revealed.push(q);
            if self.adjacent(q) == 0 {
                for r in q[0].saturating_sub(1)..=(q[0] + 1).min(self.rows() - 1) {
                    for c in q[1].saturating_sub(1)..=(q[1] + 1).min(self.cols() - 1) {
                        if !self.revealed.contains(&[r, c]) {
                            queue.push([r, c]);
                        }
                    }
                }
            }
        }
    }
    fn find_word(&mut self, a: [usize; 2], b: [usize; 2]) {
        let dr = b[0] as isize - a[0] as isize;
        let dc = b[1] as isize - a[1] as isize;
        if dr != 0 && dc != 0 && dr.abs() != dc.abs() {
            return;
        }
        let cells: Vec<[usize; 2]> = (0..=dr.abs().max(dc.abs()))
            .map(|i| {
                [
                    (a[0] as isize + i * dr.signum()) as usize,
                    (a[1] as isize + i * dc.signum()) as usize,
                ]
            })
            .collect();
        let word: String = cells
            .iter()
            .filter_map(|p| self.puzzle.data["grid"][p[0]][p[1]].as_str())
            .collect();
        let reverse: String = word.chars().rev().collect();
        let words = self.puzzle.data["words"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for w in words {
            if let Some(w) = w.as_str()
                && (w == word || w == reverse)
                && !self.found.iter().any(|f| f["word"] == w)
            {
                let mut ordered = cells.clone();
                if w != word {
                    ordered.reverse();
                }
                self.found.push(json!({"word":w,"cells":ordered}));
                self.actions += 1;
                break;
            }
        }
    }
    pub fn enter(&mut self, value: &str) {
        if self.won || self.lost {
            return;
        }
        match self.puzzle.kind.as_str() {
            "sudoku" | "mini-sudoku" => {
                if let Some([r, c]) = self.selected
                    && self.puzzle.data["grid"][r][c] == 0
                {
                    self.grid[r][c] = value.parse().unwrap_or(0);
                    self.actions += 1;
                }
            }
            "arithmetic" => {
                if value == "erase" {
                    self.text.pop();
                } else if value == "-" {
                    if self.text.starts_with('-') {
                        self.text.remove(0);
                    } else {
                        self.text.insert(0, '-');
                    }
                } else if self.text.len() < 12 {
                    self.text.push_str(value);
                }
                self.actions += 1;
            }
            "cryptogram" => {
                if let Some([i, _]) = self.selected {
                    let letters = self.cipher_letters();
                    if let Some(cipher) = letters.get(i) {
                        self.mapping.retain(|(c, _)| c != cipher);
                        if let Some(plain) = value.chars().next().filter(|c| c.is_ascii_uppercase())
                        {
                            self.mapping.retain(|(_, p)| *p != plain);
                            self.mapping.push((*cipher, plain));
                        }
                        self.actions += 1;
                    }
                }
            }
            _ => {}
        }
    }
    pub fn cipher_letters(&self) -> Vec<char> {
        let mut letters: Vec<char> = self.puzzle.data["ciphertext"]
            .as_str()
            .unwrap_or("")
            .chars()
            .filter(char::is_ascii_alphabetic)
            .collect();
        letters.sort();
        letters.dedup();
        letters
    }
    pub fn plaintext(&self) -> String {
        self.puzzle.data["ciphertext"]
            .as_str()
            .unwrap_or("")
            .chars()
            .map(|c| {
                if c.is_ascii_alphabetic() {
                    self.mapping
                        .iter()
                        .find(|(k, _)| *k == c)
                        .map_or('_', |(_, p)| *p)
                } else {
                    c
                }
            })
            .collect()
    }
    pub fn answer(&self) -> Value {
        match self.puzzle.kind.as_str() {
            "maze" => json!(self.path),
            "word-search" => json!(self.found),
            "sliding-puzzle" => json!({"moves":self.moves}),
            "lights-out" => json!(self.presses),
            "minesweeper" => json!(self.flags),
            "arithmetic" => self.text.parse::<i64>().map_or(Value::Null, |n| json!(n)),
            "cryptogram" => json!(self.plaintext()),
            _ => json!(
                self.grid
                    .iter()
                    .map(|r| r.iter().map(|n| (*n).max(0)).collect::<Vec<_>>())
                    .collect::<Vec<_>>()
            ),
        }
    }
    pub fn practice_correct(&self) -> bool {
        if self.lost {
            return false;
        }
        match self.puzzle.kind.as_str() {
            "maze" => self
                .path
                .last()
                .is_some_and(|p| json!(p) == self.puzzle.data["end"]),
            "word-search" => {
                self.found.len() == self.puzzle.data["words"].as_array().map_or(0, Vec::len)
            }
            "sliding-puzzle" => self
                .grid
                .iter()
                .flatten()
                .enumerate()
                .all(|(i, v)| *v == ((i + 1) % (self.rows() * self.cols())) as i32),
            "lights-out" => self.grid.iter().flatten().all(|v| *v == 0),
            "minesweeper" => self.puzzle.solution.as_ref().is_some_and(|s| {
                let mines = coords(s);
                mines.len() == self.flags.len() && mines.iter().all(|p| self.flags.contains(p))
            }),
            "cryptogram" => self
                .puzzle
                .solution
                .as_ref()
                .and_then(Value::as_str)
                .is_some_and(|s| self.plaintext().trim().eq_ignore_ascii_case(s.trim())),
            _ => self.puzzle.solution.as_ref() == Some(&self.answer()),
        }
    }
    pub fn hint(&mut self) -> bool {
        let Some(solution) = self.puzzle.solution.clone() else {
            return false;
        };
        if self.won || self.lost {
            return false;
        }
        let changed = match self.puzzle.kind.as_str() {
            "sudoku" | "mini-sudoku" | "nonogram" => {
                let target = (0..self.rows())
                    .flat_map(|r| (0..self.cols()).map(move |c| [r, c]))
                    .find(|[r, c]| {
                        self.grid[*r][*c].max(0) != solution[*r][*c].as_i64().unwrap_or(0) as i32
                    });
                if let Some([r, c]) = target {
                    self.grid[r][c] = solution[r][c].as_i64().unwrap_or(0) as i32;
                    self.selected = Some([r, c]);
                    true
                } else {
                    false
                }
            }
            "maze" => {
                let path = coords(&solution);
                if let Some(current) = self.path.last()
                    && let Some(i) = path.iter().position(|p| p == current)
                    && let Some(next) = path.get(i + 1)
                {
                    self.tap(*next);
                    true
                } else if self.path.len() > 1 {
                    self.path.pop();
                    true
                } else {
                    false
                }
            }
            "word-search" => {
                if let Some(word) = solution.as_array().and_then(|a| {
                    a.iter()
                        .find(|s| !self.found.iter().any(|f| f["word"] == s["word"]))
                }) {
                    self.found.push(word.clone());
                    true
                } else {
                    false
                }
            }
            "sliding-puzzle" => {
                // Reverse the last player move; the stored scramble solution starts at the original board.
                if let Some(last) = self.moves.pop() {
                    let e = (0..self.rows())
                        .flat_map(|r| (0..self.cols()).map(move |c| [r, c]))
                        .find(|[r, c]| self.grid[*r][*c] == 0)
                        .unwrap();
                    let p = match last.as_str() {
                        "up" => [e[0] + 1, e[1]],
                        "down" => [e[0] - 1, e[1]],
                        "left" => [e[0], e[1] + 1],
                        _ => [e[0], e[1] - 1],
                    };
                    self.grid[e[0]][e[1]] = self.grid[p[0]][p[1]];
                    self.grid[p[0]][p[1]] = 0;
                    true
                } else if let Some(direction) = solution["moves"][0].as_str() {
                    self.step(direction);
                    true
                } else {
                    false
                }
            }
            "lights-out" => {
                let remaining: Vec<_> = coords(&solution)
                    .into_iter()
                    .chain(self.presses.clone())
                    .fold(Vec::new(), |mut a, p| {
                        toggle(&mut a, p);
                        a
                    });
                if let Some(p) = remaining.first() {
                    self.tap(*p);
                    true
                } else {
                    false
                }
            }
            "minesweeper" => {
                if let Some(p) = coords(&solution)
                    .iter()
                    .find(|p| !self.flags.contains(p))
                    .copied()
                {
                    self.flags.push(p);
                    true
                } else if let Some(i) = self.flags.iter().position(|p| !self.mine(*p)) {
                    self.flags.remove(i);
                    true
                } else {
                    false
                }
            }
            "arithmetic" => {
                self.text = solution.to_string();
                true
            }
            "cryptogram" => {
                let cipher = self.puzzle.data["ciphertext"].as_str().unwrap_or("");
                let plain = solution.as_str().unwrap_or("");
                let pair = cipher
                    .chars()
                    .zip(plain.chars())
                    .find(|(c, p)| c.is_ascii_alphabetic() && !self.mapping.contains(&(*c, *p)));
                if let Some((c, p)) = pair {
                    self.mapping.retain(|(k, v)| *k != c && *v != p);
                    self.mapping.push((c, p));
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if changed {
            self.assisted = true;
            self.actions += 1;
        }
        changed
    }
    pub fn step(&mut self, direction: &str) {
        let point = if self.puzzle.kind == "maze" {
            self.path.last().copied()
        } else {
            (0..self.rows())
                .flat_map(|r| (0..self.cols()).map(move |c| [r, c]))
                .find(|[r, c]| self.grid[*r][*c] == 0)
        };
        if let Some([r, c]) = point {
            let (dr, dc) = match direction {
                "up" => (-1, 0),
                "down" => (1, 0),
                "left" => (0, -1),
                _ => (0, 1),
            };
            let r = r as isize + dr;
            let c = c as isize + dc;
            if r >= 0 && c >= 0 {
                self.tap([r as usize, c as usize]);
            }
        }
    }
}
fn toggle(list: &mut Vec<[usize; 2]>, p: [usize; 2]) {
    if let Some(i) = list.iter().position(|q| q == &p) {
        list.remove(i);
    } else {
        list.push(p);
    }
}

pub fn fixtures() -> Vec<Puzzle> {
    serde_json::from_str(include_str!("../public/practice.json")).expect("bundled practice puzzles")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maze_movement_rejects_walls_and_finishes_without_an_answer_key() {
        for puzzle in fixtures().into_iter().filter(|p| p.kind == "maze") {
            let route = coords(puzzle.solution.as_ref().unwrap());
            let mut game = Game::new(puzzle, true);
            game.puzzle.solution = None;
            assert!(!game.is_solved());
            let initial = game.path.clone();
            game.tap([0, 0]);
            assert_eq!(game.path, initial, "Walls must not move the player");
            for pair in route.windows(2) {
                let direction = if pair[1][0] < pair[0][0] {
                    "up"
                } else if pair[1][0] > pair[0][0] {
                    "down"
                } else if pair[1][1] < pair[0][1] {
                    "left"
                } else {
                    "right"
                };
                game.step(direction);
                assert_eq!(game.path.last(), Some(&pair[1]));
            }
            assert!(game.is_solved());
        }
    }
    #[test]
    fn available_games_finish_from_player_input_without_check_or_answer_key() {
        for puzzle in fixtures()
            .into_iter()
            .filter(|p| matches!(p.kind.as_str(), "sudoku" | "mini-sudoku" | "word-search"))
        {
            let solution = puzzle.solution.clone().unwrap();
            let mut game = Game::new(puzzle, true);
            game.puzzle.solution = None;
            assert!(!game.is_solved());
            if game.puzzle.kind == "word-search" {
                for word in solution.as_array().unwrap() {
                    let cells = coords(&word["cells"]);
                    game.tap(cells[0]);
                    game.tap(*cells.last().unwrap());
                }
            } else {
                for (r, row) in numeric_grid(&solution).iter().enumerate() {
                    for (c, digit) in row.iter().enumerate() {
                        if game.grid[r][c] == 0 {
                            game.tap([r, c]);
                            game.enter(&digit.to_string());
                        }
                    }
                }
            }
            assert!(
                game.is_solved(),
                "{} {}",
                game.puzzle.kind,
                game.puzzle.difficulty
            );
            if game.puzzle.kind != "word-search" {
                game.grid[0][0] = game.grid[0][1];
                assert!(!game.is_solved(), "Full invalid Sudoku must not win");
            }
        }
    }
    #[test]
    fn every_fixture_matches_engine() {
        for p in fixtures() {
            let request = puzzgen::puzzles::GenerateRequest {
                kind: serde_json::from_value(json!(p.kind)).unwrap(),
                seed: Some(p.seed.parse().unwrap()),
                difficulty: serde_json::from_value(json!(p.difficulty)).unwrap(),
                include_solution: false,
            };
            assert!(
                puzzgen::puzzles::verify(request, p.solution.clone().unwrap()).unwrap(),
                "{} {}",
                p.kind,
                p.difficulty
            );
        }
    }
    #[test]
    fn playable_solutions_for_all_families() {
        for p in fixtures() {
            let mut g = Game::new(p.clone(), false);
            let s = p.solution.unwrap();
            match p.kind.as_str() {
                "sudoku" | "mini-sudoku" | "nonogram" => g.grid = numeric_grid(&s),
                "maze" => {
                    for p in coords(&s).into_iter().skip(1) {
                        g.tap(p);
                    }
                }
                "word-search" => {
                    for w in s.as_array().unwrap() {
                        let c = coords(&w["cells"]);
                        g.tap(c[0]);
                        g.tap(*c.last().unwrap());
                    }
                }
                "sliding-puzzle" => {
                    for d in s["moves"].as_array().unwrap() {
                        g.step(d.as_str().unwrap());
                    }
                }
                "lights-out" => {
                    for p in coords(&s) {
                        g.tap(p);
                    }
                }
                "minesweeper" => {
                    g.alternate = true;
                    for p in coords(&s) {
                        g.tap(p);
                    }
                }
                "arithmetic" => g.text = s.to_string(),
                "cryptogram" => {
                    g.mapping = g.puzzle.data["ciphertext"]
                        .as_str()
                        .unwrap()
                        .chars()
                        .zip(s.as_str().unwrap().chars())
                        .filter(|(c, _)| c.is_ascii_alphabetic())
                        .collect()
                }
                _ => unreachable!(),
            };
            assert!(g.practice_correct(), "{} {}", p.kind, p.difficulty);
            let request = puzzgen::puzzles::GenerateRequest {
                kind: serde_json::from_value(json!(g.puzzle.kind)).unwrap(),
                seed: Some(g.puzzle.seed.parse().unwrap()),
                difficulty: serde_json::from_value(json!(g.puzzle.difficulty)).unwrap(),
                include_solution: false,
            };
            assert!(
                puzzgen::puzzles::verify(request, g.answer()).unwrap(),
                "native answer format {}",
                g.puzzle.kind
            );
        }
    }
    #[test]
    fn lights_parity_and_sliding_illegal_move() {
        let f = fixtures();
        let mut g = Game::new(
            f.iter().find(|p| p.kind == "lights-out").unwrap().clone(),
            false,
        );
        let initial = g.grid.clone();
        g.tap([0, 0]);
        g.tap([0, 0]);
        assert_eq!(initial, g.grid);
        assert!(g.presses.is_empty());
        let mut g = Game::new(
            f.iter()
                .find(|p| p.kind == "sliding-puzzle")
                .unwrap()
                .clone(),
            false,
        );
        let initial = g.grid.clone();
        g.tap([999, 999]);
        assert_eq!(initial, g.grid);
    }
    #[test]
    fn mines_reveal_and_flag_safety() {
        let mut g = Game::new(
            fixtures()
                .into_iter()
                .find(|p| p.kind == "minesweeper")
                .unwrap(),
            false,
        );
        let mine = coords(g.puzzle.solution.as_ref().unwrap())[0];
        g.alternate = true;
        g.tap(mine);
        g.alternate = false;
        g.tap(mine);
        assert!(!g.lost);
        g.alternate = true;
        g.tap(mine);
        g.alternate = false;
        g.tap(mine);
        assert!(g.lost);
        assert!(!g.practice_correct());
    }
}
