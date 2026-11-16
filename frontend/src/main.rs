#![recursion_limit = "512"]

mod account;
mod discovery;
mod language;
mod learn;

use gloo_net::http::Request;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wasm_bindgen_futures::{JsFuture, spawn_local};

const KINDS: [(&str, &str, &str, &str); 21] = [
    (
        "spoonerism",
        "Spoonerisms",
        "↝",
        "Swap sounds. Discover a new phrase.",
    ),
    ("anagram", "Anagrams", "Ab", "A familiar word, rearranged"),
    (
        "word-ladder",
        "Word ladders",
        "⇧",
        "One letter changes the journey",
    ),
    (
        "compound-bridge",
        "Compound bridges",
        "⌁",
        "One word connects two ideas",
    ),
    (
        "word-sorting",
        "Word sorting",
        "⊂",
        "Find the shared category",
    ),
    (
        "sequence-detective",
        "Sequence detective",
        "…",
        "Discover the missing number",
    ),
    ("memory-pairs", "Memory pairs", "◈", "Remember and reunite"),
    (
        "path-connections",
        "Path connections",
        "┼",
        "Rotate a living network",
    ),
    (
        "futoshiki",
        "Futoshiki",
        "<",
        "Numbers with a sense of order",
    ),
    (
        "sound-families",
        "Sound families",
        "♪",
        "Find the rhyming sounds",
    ),
    (
        "sentence-assembly",
        "Sentence assembly",
        "¶",
        "Give words a place to belong",
    ),
    ("sudoku", "Sudoku", "09", "The classic number challenge"),
    (
        "mini-sudoku",
        "Mini Sudoku",
        "04",
        "A smaller grid, a quick reset",
    ),
    ("maze", "Maze", "↳", "Find your way from A to B"),
    (
        "nonogram",
        "Nonogram",
        "▦",
        "Follow the clues. Find the picture.",
    ),
    (
        "word-search",
        "Word search",
        "Aa",
        "Hidden words in every direction",
    ),
    (
        "sliding-puzzle",
        "Sliding puzzle",
        "↔",
        "A little room to rearrange",
    ),
    (
        "lights-out",
        "Lights out",
        "✳",
        "One switch changes everything",
    ),
    (
        "minesweeper",
        "Minesweeper",
        "✣",
        "A field of calculated decisions",
    ),
    (
        "arithmetic",
        "Arithmetic",
        "±",
        "Give your mental maths a workout",
    ),
    (
        "cryptogram",
        "Cryptogram",
        "?",
        "Crack a secret substitution",
    ),
];

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Puzzle {
    id: String,
    generator_version: String,
    kind: String,
    seed: String,
    difficulty: String,
    data: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    solution: Option<Value>,
    rules: String,
    uniqueness: String,
}

fn title(kind: &str) -> &'static str {
    KINDS
        .iter()
        .find(|item| item.0 == kind)
        .map_or("Puzzle", |item| item.1)
}

fn grid(value: &Value) -> Vec<Vec<u8>> {
    value
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    row.as_array()
                        .map(|cells| {
                            cells
                                .iter()
                                .map(|cell| cell.as_u64().unwrap_or(0) as u8)
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn coords(value: &Value) -> Vec<(usize, usize)> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    Some((
                        item.get(0)?.as_u64()? as usize,
                        item.get(1)?.as_u64()? as usize,
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn api_error(value: &Value, status: u16) -> String {
    value
        .pointer("/error/message")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("The API returned HTTP {status}. Please try again."))
}

async fn generate(
    kind: String,
    difficulty: String,
    seed: String,
    key: String,
    include_solution: bool,
    csrf: String,
    content_version: Option<String>,
) -> Result<Puzzle, String> {
    let mut body = json!({"kind":kind,"difficulty":difficulty,"include_solution":include_solution});
    if let Some(version) = content_version {
        body["content_version"] = json!(version);
    }
    if !seed.trim().is_empty() {
        let seed = seed
            .trim()
            .parse::<u64>()
            .map_err(|_| "Use a whole-number seed from 0 to 18446744073709551615.".to_string())?;
        body["seed"] = json!(seed);
    }
    let mut request = Request::post("/api/v1/generate");
    if !csrf.is_empty() {
        request = request.header("X-CSRF-Token", &csrf);
    }
    if !key.trim().is_empty() {
        request = request.header("Authorization", &format!("Bearer {}", key.trim()));
    }
    let response = request
        .json(&body)
        .map_err(|error| error.to_string())?
        .send()
        .await
        .map_err(|_| {
            "Could not reach the API. Check that the Rust server is running.".to_string()
        })?;
    let status = response.status();
    let value: Value = response
        .json()
        .await
        .map_err(|_| "The API returned an unreadable response.".to_string())?;
    if !response.ok() {
        return Err(api_error(&value, status));
    }
    serde_json::from_value(value).map_err(|error| format!("Unexpected puzzle format: {error}"))
}

async fn verify(puzzle: Puzzle, answer: Value, key: String, csrf: String) -> Result<bool, String> {
    let seed = puzzle
        .seed
        .parse::<u64>()
        .map_err(|_| "This puzzle has an invalid seed.".to_string())?;
    let mut request = Request::post("/api/v1/verify");
    if !csrf.is_empty() {
        request = request.header("X-CSRF-Token", &csrf);
    }
    if !key.trim().is_empty() {
        request = request.header("Authorization", &format!("Bearer {}", key.trim()));
    }
    let mut body =
        json!({"kind":puzzle.kind,"difficulty":puzzle.difficulty,"seed":seed,"answer":answer});
    if let Some(version) = puzzle.data["content_version"].as_str() {
        body["content_version"] = json!(version);
    }
    let response = request
        .json(&body)
        .map_err(|error| error.to_string())?
        .send()
        .await
        .map_err(|_| "Could not reach the API to check your answer.".to_string())?;
    let status = response.status();
    let value: Value = response
        .json()
        .await
        .map_err(|_| "The API returned an unreadable response.".to_string())?;
    if !response.ok() {
        return Err(api_error(&value, status));
    }
    value["correct"]
        .as_bool()
        .ok_or_else(|| "The API returned an unexpected answer check.".to_string())
}

#[derive(Clone, Copy)]
struct Workbench {
    puzzle: RwSignal<Option<Puzzle>>,
    busy: RwSignal<bool>,
    error: RwSignal<String>,
    revealed: RwSignal<bool>,
    edits: RwSignal<Vec<Vec<u8>>>,
    answer: RwSignal<String>,
    result: RwSignal<String>,
    key: RwSignal<String>,
    csrf: RwSignal<String>,
    status: RwSignal<String>,
}

impl Workbench {
    fn load(self, kind: String, difficulty: String, seed: String) {
        self.busy.set(true);
        // A previous board must not remain under a newly selected puzzle title,
        // including when the next request fails.
        self.puzzle.set(None);
        self.edits.set(Vec::new());
        self.error.set(String::new());
        self.result.set(String::new());
        self.revealed.set(false);
        self.answer.set(
            match kind.as_str() {
                "word-sorting" => "{}",
                "memory-pairs" | "sound-families" | "sentence-assembly" => "[]",
                _ => "",
            }
            .to_owned(),
        );
        spawn_local(async move {
            match generate(
                kind,
                difficulty,
                seed,
                self.key.get_untracked(),
                false,
                self.csrf.get_untracked(),
                None,
            )
            .await
            {
                Ok(puzzle) => {
                    self.status.set("API online".into());
                    self.edits.set(grid(&puzzle.data["grid"]));
                    self.puzzle.set(Some(puzzle));
                }
                Err(error) => self.error.set(error),
            }
            self.busy.set(false);
        });
    }

    fn reveal(self) {
        if self.revealed.get_untracked() {
            self.revealed.set(false);
            return;
        }
        let Some(puzzle) = self.puzzle.get_untracked() else {
            return;
        };
        if puzzle.solution.is_some() {
            self.revealed.set(true);
            return;
        }
        self.busy.set(true);
        self.error.set(String::new());
        spawn_local(async move {
            match generate(
                puzzle.kind,
                puzzle.difficulty,
                puzzle.seed,
                self.key.get_untracked(),
                true,
                self.csrf.get_untracked(),
                puzzle.data["content_version"].as_str().map(str::to_owned),
            )
            .await
            {
                Ok(puzzle) => {
                    self.puzzle.set(Some(puzzle));
                    self.revealed.set(true);
                }
                Err(error) => self.error.set(error),
            }
            self.busy.set(false);
        });
    }

    fn check(self) {
        let Some(puzzle) = self.puzzle.get_untracked() else {
            return;
        };
        let answer = if puzzle.kind == "sudoku"
            || puzzle.kind == "mini-sudoku"
            || puzzle.kind == "nonogram"
            || puzzle.kind == "futoshiki"
            || puzzle.kind == "path-connections"
        {
            json!(self.edits.get_untracked())
        } else if ["cryptogram", "spoonerism", "anagram", "compound-bridge"]
            .contains(&puzzle.kind.as_str())
        {
            json!(self.answer.get_untracked())
        } else if puzzle.kind == "word-ladder" {
            json!(
                self.answer
                    .get_untracked()
                    .split_whitespace()
                    .collect::<Vec<_>>()
            )
        } else {
            match serde_json::from_str::<Value>(&self.answer.get_untracked()) {
                Ok(value) => value,
                Err(_) => {
                    self.result
                        .set("Enter an answer in the JSON format shown below.".into());
                    return;
                }
            }
        };
        self.busy.set(true);
        self.result.set(String::new());
        spawn_local(async move {
            self.result.set(
                match verify(
                    puzzle,
                    answer,
                    self.key.get_untracked(),
                    self.csrf.get_untracked(),
                )
                .await
                {
                    Ok(true) => "Nicely solved! Your answer is correct.".into(),
                    Ok(false) => "Not quite yet. Keep going — you can try again.".into(),
                    Err(error) => error,
                },
            );
            self.busy.set(false);
        });
    }
}

#[component]
fn App() -> impl IntoView {
    let account_page = web_sys::window()
        .is_some_and(|window| window.location().pathname().ok().as_deref() == Some("/account"));
    let initial_kind = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|query| {
            query.trim_start_matches('?').split('&').find_map(|part| {
                part.strip_prefix("kind=")
                    .filter(|kind| KINDS.iter().any(|item| item.0 == *kind))
                    .map(str::to_owned)
            })
        })
        .unwrap_or_else(|| "sudoku".to_owned());
    let kind = RwSignal::new(initial_kind);
    let difficulty = RwSignal::new("medium".to_string());
    let seed = RwSignal::new(String::new());
    let status = RwSignal::new("Connecting".to_string());
    let show_json = RwSignal::new(false);
    let copy_status = RwSignal::new(String::new());
    let state = Workbench {
        puzzle: RwSignal::new(None),
        busy: RwSignal::new(false),
        error: RwSignal::new(String::new()),
        revealed: RwSignal::new(false),
        edits: RwSignal::new(Vec::new()),
        answer: RwSignal::new(String::new()),
        result: RwSignal::new(String::new()),
        key: RwSignal::new(String::new()),
        csrf: RwSignal::new(String::new()),
        status,
    };

    spawn_local(async move {
        match Request::get("/api/v1/catalog").send().await {
            Ok(response) if response.ok() => {
                status.set("API online".into());
                let managed = Request::get("/api/v1/account/config").send().await.ok();
                let managed = if let Some(response) = managed {
                    response
                        .json::<Value>()
                        .await
                        .ok()
                        .is_some_and(|v| v["enabled"] == true)
                } else {
                    false
                };
                if managed
                    && let Ok(response) = Request::get("/api/v1/account/session").send().await
                    && response.ok()
                    && let Ok(session) = response.json::<Value>().await
                {
                    state
                        .csrf
                        .set(session["csrf_token"].as_str().unwrap_or("").into());
                }
                if !managed && !account_page {
                    state.load(kind.get_untracked(), "medium".into(), String::new());
                }
            }
            Ok(response) if response.status() == 401 => {
                status.set("API key required".into());
                state
                    .error
                    .set("Add your API key in Connection, then generate a puzzle.".into());
            }
            _ => {
                status.set("API unavailable".into());
                state
                    .error
                    .set("Start the Rust API on port 8080, then generate a puzzle.".into());
            }
        }
    });

    view! {
        <div class="site-shell">
            <header class="topbar">
                <a class="brand" href="/" aria-label="Puzzgen home"><span class="brand-icon" aria-hidden="true">"p"</span>"puzzgen"<span class="brand-dot">"."</span></a>
                <nav aria-label="Main navigation"><a class="nav-current" href="/#workbench">"Playground"</a><a href="/#learn">"Learn"</a><a href="/account">"Account"</a><a href="/docs" target="_blank" rel="noreferrer">"API reference"<span aria-hidden="true">" ↗"</span></a></nav>
                <span class="api-status"><span class=move || if status.get() == "API online" { "status-dot" } else { "status-dot pending" }></span>{move || status.get()}</span>
            </header>

            <main>
                <Show when=move ||!account_page>
                <section class="hero" aria-labelledby="hero-title">
                    <div><div class="eyebrow"><span class="tiny-square"></span>"A FREE PLACE TO LEARN BY BUILDING"</div><h1 id="hero-title">"Small games."<br/><span>"Big discoveries."</span></h1><p class="hero-description">"Practice interfaces, explore puzzle logic, and build your first game. We provide the puzzles and a free API. You bring your curiosity."</p><a class="hero-learn" href="#learn">"Start building →"</a></div>
                    <div class="hero-aside"><div class="orbit-mark" aria-hidden="true"><span>"✳"</span></div><p>"Personally hosted."<br/><span>"Free to learn. Made to share."</span></p></div>
                </section>

                <section class="workbench" id="workbench" aria-label="Puzzle playground">
                    <aside class="sidebar">
                        <div class="section-label"><span>"01 / PICK A PUZZLE"</span><span class="count">"21"</span></div>
                        <div class="kind-list">{KINDS.into_iter().map(|(slug, name, icon, _)| view! {
                            <button class=move || if kind.get() == slug { "kind-button selected" } else { "kind-button" } aria-pressed=move || (kind.get() == slug).to_string() disabled=move || state.busy.get() on:click=move |_| { kind.set(slug.into()); copy_status.set(String::new()); state.load(slug.into(), difficulty.get_untracked(), seed.get_untracked()); }><span class="kind-icon" aria-hidden="true">{icon}</span><span>{name}</span><span class="kind-arrow" aria-hidden="true">"↗"</span></button>
                        }).collect_view()}</div>
                        <div class="sidebar-note"><span class="note-icon" aria-hidden="true">"↻"</span><p><strong>"One seed. Same puzzle."</strong><br/>"Share a seed to share the challenge. Leave it blank for something new."</p></div>
                        <details class="connection"><summary>"Connection"</summary><label for="api-key">"API key (optional)"</label><input id="api-key" type="password" autocomplete="off" placeholder="Bearer key" prop:value=move || state.key.get() on:input=move |ev| state.key.set(event_target_value(&ev))/><p>"Kept in memory for this tab only."</p></details>
                    </aside>

                    <div class="workspace">
                        <div class="controls"><div class="puzzle-heading"><span class="section-label">"02 / MAKE IT YOURS"</span><h2>{move || title(&kind.get())}</h2><p>{move || KINDS.iter().find(|item| item.0 == kind.get()).map_or("", |item| item.3)}</p></div>
                            <form class="generation-form" on:submit=move |ev| { ev.prevent_default(); copy_status.set(String::new()); state.load(kind.get(), difficulty.get(), seed.get()); }>
                                <div class="field"><label for="difficulty">"Difficulty"</label><select id="difficulty" disabled=move || state.busy.get() prop:value=move || difficulty.get() on:change=move |ev| difficulty.set(event_target_value(&ev))><option value="easy">"Easy"</option><option value="medium">"Medium"</option><option value="hard">"Hard"</option></select></div>
                                <div class="field seed-field"><label for="seed">"Seed "<span>"/ optional"</span></label><input id="seed" type="text" inputmode="numeric" pattern="[0-9]*" placeholder="Surprise me" disabled=move || state.busy.get() prop:value=move || seed.get() on:input=move |ev| seed.set(event_target_value(&ev))/></div>
                                <button class="generate-button" type="submit" disabled=move || state.busy.get()><span aria-hidden="true">"✳"</span>{move || if state.busy.get() { "Generating…" } else { "Generate puzzle" }}</button>
                            </form>
                        </div>

                        <Show when=move || !state.error.get().is_empty()><div class="error-banner" role="alert">{move || state.error.get()}</div></Show>

                        <div class="puzzle-area" aria-busy=move || state.busy.get().to_string()>
                            {move || state.puzzle.get().map(|puzzle| {
                                let label = title(&puzzle.kind);
                                let puzzle_for_panel = puzzle.clone();
                                view! {
                                    <div class="board-header"><div class="board-title"><span class="board-dot"></span><span>{label}</span><span class="difficulty-badge">{puzzle.difficulty.clone()}</span></div><button class="text-button" disabled=move || state.busy.get() on:click=move |_| state.reveal()>{move || if state.revealed.get() { "Hide solution" } else { "Reveal solution" }}<span aria-hidden="true">" ↗"</span></button></div>
                                    <PuzzleBoard puzzle=puzzle_for_panel state=state/>
                                    <div class="board-footer"><span>"SEED "<strong>{puzzle.seed.clone()}</strong></span><span>{puzzle.generator_version.clone()}" · deterministic"</span></div>
                                    <div class="rules"><span class="rules-marker" aria-hidden="true">"i"</span><div><p>{puzzle.rules.clone()}</p><small>{puzzle.uniqueness.clone()}</small></div></div>
                                }.into_any()
                            }).unwrap_or_else(|| view! { <div class="empty-state"><span aria-hidden="true">"✳"</span><h3>"Your next challenge is waiting."</h3><p>"Choose a puzzle and select Generate puzzle to begin."</p></div> }.into_any())}
                        </div>

                        <Show when=move || state.puzzle.get().is_some()>
                            <div class="answer-section">
                                <div class="answer-heading"><div><span class="section-label">"YOUR MOVE"</span><p>{move || match state.puzzle.get().map(|p| p.kind).as_deref() { Some("sudoku" | "mini-sudoku") => "Fill the empty cells, then check your answer.", Some("nonogram") => "Tap cells to fill the picture, then check your answer.", Some("cryptogram") => "Enter the decoded sentence below.", Some("spoonerism" | "anagram" | "compound-bridge") => "Use the clue, then enter your answer below.", Some("word-ladder") => "Build a path from the first word to the last.", Some("word-sorting" | "sequence-detective" | "memory-pairs" | "path-connections" | "futoshiki" | "sound-families" | "sentence-assembly") => "Use the controls on the board, then check your answer.", _ => "Try an answer using the format below." }}</p></div><button class="check-button" disabled=move || state.busy.get() || state.revealed.get() on:click=move |_| state.check()>"Check answer "<span aria-hidden="true">"→"</span></button></div>
                                <Show when=move || state.puzzle.get().is_some_and(|p| !["sudoku", "mini-sudoku", "nonogram", "word-sorting", "sequence-detective", "memory-pairs", "path-connections", "futoshiki", "sound-families", "sentence-assembly"].contains(&p.kind.as_str()))><label class="sr-only" for="answer">"Your answer"</label><textarea id="answer" rows="2" placeholder=move || answer_example(state.puzzle.get().map(|p| p.kind).as_deref().unwrap_or("")) prop:value=move || state.answer.get() on:input=move |ev| { state.answer.set(event_target_value(&ev)); state.result.set(String::new()); }></textarea><p class="format-hint">{move || answer_hint(state.puzzle.get().map(|p| p.kind).as_deref().unwrap_or(""))}</p></Show>
                                <Show when=move || !state.result.get().is_empty()><p class="answer-result" role="status">{move || state.result.get()}</p></Show>
                            </div>

                            <div class="developer-panel"><button class="json-toggle" aria-expanded=move || show_json.get().to_string() on:click=move |_| show_json.update(|show| *show = !*show)><span class="code-icon" aria-hidden="true">"{ }"</span><span>"Under the hood"</span><span class="json-subtitle">"Explore the JSON response"</span><span aria-hidden="true">{move || if show_json.get() { "−" } else { "+" }}</span></button>
                                <Show when=move || show_json.get()><div class="json-content"><div class="json-meta"><code>"POST /api/v1/generate"</code><button class="text-button" on:click=move |_| { if let Some(mut puzzle) = state.puzzle.get_untracked() { if !state.revealed.get_untracked() { puzzle.solution = None; } let text = serde_json::to_string_pretty(&puzzle).unwrap_or_default(); spawn_local(async move { if let Some(window) = web_sys::window() { let result = JsFuture::from(window.navigator().clipboard().write_text(&text)).await; copy_status.set(if result.is_ok() { "Copied" } else { "Select the JSON to copy" }.into()); } }); } }>{move || if copy_status.get().is_empty() { "Copy JSON".into() } else { copy_status.get() }}</button></div><pre><code>{move || state.puzzle.get().map(|mut puzzle| { if !state.revealed.get() { puzzle.solution = None; } serde_json::to_string_pretty(&puzzle).unwrap_or_default() }).unwrap_or_default()}</code></pre></div></Show>
                            </div>
                        </Show>
                    </div>
                </section>

                <section class="api-strip" aria-label="Our mission"><div class="api-strip-title"><span class="code-icon" aria-hidden="true">"{ }"</span><div><h3>"Build the game you wish existed."</h3><p>"A personally funded learning resource. Free member accounts with fair limits. Offline examples need no account."</p></div></div><a href="#learn">"Learn by doing "<span aria-hidden="true">"↗"</span></a></section>
                <learn::LearningHub/>
                </Show>
                <Show when=move ||account_page><account::AccountPanel csrf=state.csrf/></Show>
            </main>
            <footer><a class="footer-brand" href="/">"puzzgen."</a><p>"Free puzzles. Shared knowledge. Your next small victory."</p><span><a href="https://www.rust-lang.org/" target="_blank" rel="noreferrer">"Rust"</a>" · "<a href="https://github.com/tokio-rs/axum" target="_blank" rel="noreferrer">"Axum"</a>" · "<a href="https://leptos.dev/" target="_blank" rel="noreferrer">"Leptos"</a></span></footer>
        </div>
    }
}

fn answer_example(kind: &str) -> &'static str {
    match kind {
        "arithmetic" => "42",
        "spoonerism" => "Type the new two-word phrase…",
        "anagram" | "compound-bridge" => "Type your word…",
        "word-ladder" => "cold cord card ward warm",
        "cryptogram" => "Type the decoded sentence…",
        "sliding-puzzle" => "{\"moves\":[\"up\",\"left\"]}",
        "word-search" => "[{\"word\":\"RUST\",\"cells\":[[0,0],[0,1],[0,2],[0,3]]}]",
        _ => "[[0,0],[0,1],[1,1]]",
    }
}

fn answer_hint(kind: &str) -> &'static str {
    match kind {
        "arithmetic" => "Enter a number. Follow normal arithmetic order of operations.",
        "spoonerism" | "anagram" | "compound-bridge" => {
            "Type plain text, without JSON quotes. Case and extra spaces are ignored."
        }
        "word-ladder" => {
            "Enter the whole path, including start and end, separated by spaces or new lines. Or use the word buttons above."
        }
        "cryptogram" => "Use the original sentence in plain text, without JSON quotes.",
        "sliding-puzzle" => {
            "JSON object with a moves array. Directions describe movement of the empty tile."
        }
        "lights-out" => "JSON array of [row, column] switches to press. Coordinates start at zero.",
        "minesweeper" => {
            "JSON array of every mine's [row, column]. This is a board generator, not a no-guess guarantee."
        }
        "word-search" => {
            "JSON array with a word and its ordered cells for each hidden word. Coordinates start at zero."
        }
        "maze" => {
            "JSON array of [row, column] cells forming a path from start to end. Coordinates start at zero."
        }
        _ => "Coordinates start at zero.",
    }
}

#[component]
fn PuzzleBoard(puzzle: Puzzle, state: Workbench) -> impl IntoView {
    let data = puzzle.data.clone();
    let solution = puzzle.solution.clone().unwrap_or(Value::Null);
    match puzzle.kind.as_str() {
        "word-sorting" | "sequence-detective" | "memory-pairs" | "path-connections"
        | "futoshiki" | "sound-families" | "sentence-assembly" => {
            view! { <discovery::DiscoveryBoard puzzle=puzzle state=state/> }.into_any()
        }
        "sudoku" | "mini-sudoku" => {
            let givens = grid(&data["grid"]);
            let solved = grid(&solution);
            let size = givens.len();
            let box_size = data["box_size"].as_u64().unwrap_or(3) as usize;
            view! { <div class="board-center"><div class="sudoku-board" style=format!("grid-template-columns:repeat({size},1fr)") role="group" aria-label="Sudoku puzzle">
                {givens.iter().enumerate().flat_map(|(row, cells)| cells.iter().enumerate().map(move |(col, &given)| (row,col,given))).map(|(row,col,given)| {
                    let solved_value = solved.get(row).and_then(|cells| cells.get(col)).copied().unwrap_or(0);
                    let class = format!("sudoku-cell {} {} {}", if given > 0 { "given" } else { "editable" }, if (col + 1) % box_size == 0 && col + 1 < size { "box-right" } else { "" }, if (row + 1) % box_size == 0 && row + 1 < size { "box-bottom" } else { "" });
                    view! { <input class=class type="text" inputmode="numeric" maxlength="1" aria-label=format!("Row {}, column {}{}",row+1,col+1,if given>0 {", given"} else {""}) readonly=move || { given > 0 || state.revealed.get() || state.busy.get() } prop:value=move || {
                        let value = if state.revealed.get() { solved_value } else if given > 0 { given } else { state.edits.get().get(row).and_then(|cells| cells.get(col)).copied().unwrap_or(0) };
                        if value == 0 { String::new() } else { value.to_string() }
                    } on:input=move |ev| { let value = event_target_value(&ev).parse::<u8>().unwrap_or(0); state.edits.update(|rows| { if let Some(cell) = rows.get_mut(row).and_then(|cells| cells.get_mut(col)) { *cell = if value as usize <= size { value } else { 0 }; } }); state.result.set(String::new()); }/> }
                }).collect_view()}
            </div><p class="board-caption">{move || if state.revealed.get() { "Solution revealed. A fresh puzzle is one click away." } else { "A quiet moment. A good challenge. Your move." }}</p></div> }.into_any()
        }
        "nonogram" => {
            let rows = data["rows"].as_u64().unwrap_or(5) as usize;
            let cols = data["cols"].as_u64().unwrap_or(5) as usize;
            let row_clues = data["row_clues"].as_array().cloned().unwrap_or_default();
            let col_clues = data["column_clues"].as_array().cloned().unwrap_or_default();
            let solved = grid(&solution);
            if state.edits.get_untracked().is_empty() {
                state.edits.set(vec![vec![0; cols]; rows]);
            }
            let row_views = (0..rows).map(|row| {
                let clue = row_clues.get(row).and_then(Value::as_array).map(|nums| nums.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ")).unwrap_or_default();
                let buttons = (0..cols).map(|col| {
                    let solved_value = solved.get(row).and_then(|cells| cells.get(col)).copied().unwrap_or(0);
                    view! {
                        <button class=move || {
                            let filled = if state.revealed.get() { solved_value > 0 } else { state.edits.get().get(row).and_then(|cells| cells.get(col)).copied().unwrap_or(0) > 0 };
                            if filled { "nonogram-cell filled" } else { "nonogram-cell" }
                        }
                        aria-label=format!("Row {}, column {}", row + 1, col + 1)
                        aria-pressed=move || state.edits.get().get(row).and_then(|cells| cells.get(col)).is_some_and(|value| *value > 0).to_string()
                        disabled=move || state.revealed.get() || state.busy.get()
                        on:click=move |_| {
                            state.edits.update(|rows| if let Some(value) = rows.get_mut(row).and_then(|cells| cells.get_mut(col)) { *value = 1 - *value; });
                            state.result.set(String::new());
                        }></button>
                    }
                }).collect_view();
                view! { <div class="row-clue">{clue}</div>{buttons} }
            }).collect_view();
            view! { <div class="board-center"><div class="nonogram-board" style=format!("grid-template-columns: minmax(36px, auto) repeat({cols}, 1fr)")><div></div>
                {col_clues.into_iter().map(|clue| view!{<div class="column-clue">{clue.as_array().map(|nums| nums.iter().map(|num| view!{<span>{num.to_string()}</span>}).collect_view()).unwrap_or_default()}</div>}).collect_view()}
                {row_views}
            </div><p class="board-caption">"Numbers show runs of filled cells. Leave a gap between runs."</p></div>}.into_any()
        }
        "maze" => {
            let cells = grid(&data["grid"]);
            let size = cells.first().map_or(1, Vec::len);
            let start = coords(&json!([data["start"]])).first().copied();
            let end = coords(&json!([data["end"]])).first().copied();
            let path = coords(&solution);
            view!{<div class="board-center"><div class="maze-board" style=format!("grid-template-columns:repeat({size},1fr)") aria-label="Maze puzzle">
                {cells.into_iter().enumerate().flat_map(|(row,cells)|cells.into_iter().enumerate().map(move |(col,wall)|(row,col,wall))).map(|(row,col,wall)|{let on_path=path.contains(&(row,col));view!{<span class=move ||format!("maze-cell {} {}",if wall>0{"wall"}else{""},if state.revealed.get()&&on_path{"path"}else{""})>{if start==Some((row,col)){"A"}else if end==Some((row,col)){"B"}else{""}}</span>}}).collect_view()}
            </div><p class="board-caption">"A → B. Move through open cells, one step at a time."</p></div>}.into_any()
        }
        "word-search" => {
            let rows = data["grid"].as_array().cloned().unwrap_or_default();
            let size = rows.first().and_then(Value::as_array).map_or(1, Vec::len);
            let marked = solution
                .as_array()
                .map(|words| {
                    words
                        .iter()
                        .flat_map(|word| coords(&word["cells"]))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let words = data["words"].as_array().cloned().unwrap_or_default();
            view!{<div class="board-center"><div class="word-board" style=format!("grid-template-columns:repeat({size},1fr)")>{rows.into_iter().enumerate().flat_map(|(row,cells)|cells.as_array().cloned().unwrap_or_default().into_iter().enumerate().map(move |(col,cell)|(row,col,cell))).map(|(row,col,cell)|{let highlight=marked.contains(&(row,col));view!{<span class=move ||if state.revealed.get()&&highlight{"letter-cell highlighted"}else{"letter-cell"}>{cell.as_str().unwrap_or("").to_owned()}</span>}}).collect_view()}</div><div class="word-list">{words.into_iter().map(|word|view!{<span>{word.as_str().unwrap_or("").to_owned()}</span>}).collect_view()}</div></div>}.into_any()
        }
        "sliding-puzzle" | "lights-out" => {
            let cells = grid(&data["grid"]);
            let size = cells.len();
            let is_lights = puzzle.kind == "lights-out";
            let solved = coords(&solution);
            view!{<div class="board-center"><div class=if is_lights{"lights-board"}else{"sliding-board"} style=format!("grid-template-columns:repeat({size},1fr)")>{cells.into_iter().enumerate().flat_map(|(row,cells)|cells.into_iter().enumerate().map(move |(col,value)|(row,col,value))).map(|(row,col,value)|{let highlight=solved.contains(&(row,col));view!{<span class=move ||if is_lights{format!("light-cell {} {}",if value>0{"lit"}else{""},if state.revealed.get()&&highlight{"press"}else{""})}else{format!("sliding-cell {}",if value==0{"empty"}else{""})}>{if is_lights||value==0{String::new()}else{value.to_string()}}</span>}}).collect_view()}</div><p class="board-caption">{if is_lights{"Pressing a cell toggles it and its direct neighbours."}else{"Slide tiles into the empty space. Directions move the empty tile."}}</p><SolutionText solution=solution state=state/></div>}.into_any()
        }
        "minesweeper" => {
            let rows = data["rows"].as_u64().unwrap_or(8) as usize;
            let cols = data["cols"].as_u64().unwrap_or(8) as usize;
            let revealed = data["initial_revealed"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let mines = coords(&solution);
            let count = data["mine_count"].to_string();
            view!{<div class="board-center"><div class="mine-board" style=format!("grid-template-columns:repeat({cols},1fr)")>{(0..rows).flat_map(|row|(0..cols).map(move |col|(row,col))).map(|(row,col)|{let mine=mines.contains(&(row,col));let adjacent=revealed.iter().find(|cell|cell["row"].as_u64()==Some(row as u64)&&cell["col"].as_u64()==Some(col as u64)).and_then(|cell|cell["adjacent"].as_u64());view!{<span class=move ||format!("mine-cell {} {}",if adjacent.is_some(){"opened"}else{""},if state.revealed.get()&&mine{"mine"}else{""})>{move ||if state.revealed.get()&&mine{"✣".into()}else{adjacent.filter(|value|*value>0).map(|value|value.to_string()).unwrap_or_default()}}</span>}}).collect_view()}</div><p class="board-caption">{count}" mines. Your first opening is already revealed."</p></div>}.into_any()
        }
        "spoonerism" | "anagram" | "word-ladder" | "compound-bridge" => {
            view! { <language::LanguageBoard puzzle=puzzle state=state/> }.into_any()
        }
        "arithmetic" | "cryptogram" => {
            let arithmetic = puzzle.kind == "arithmetic";
            let text = data[if arithmetic {
                "expression"
            } else {
                "ciphertext"
            }]
            .as_str()
            .unwrap_or("")
            .to_owned();
            view!{<div class="board-center text-puzzle"><span class="text-puzzle-label">{if arithmetic{"SOLVE THE EXPRESSION"}else{"DECODE THE MESSAGE"}}</span><div class=if arithmetic{"expression"}else{"ciphertext"}>{text}</div><SolutionText solution=solution state=state/><p class="board-caption">{if arithmetic{"Take your time. Every operation counts."}else{"Each letter stands for another. Look for the patterns."}}</p></div>}.into_any()
        }
        _ => {
            view! {<pre>{serde_json::to_string_pretty(&data).unwrap_or_default()}</pre>}.into_any()
        }
    }
}

#[component]
fn SolutionText(solution: Value, state: Workbench) -> impl IntoView {
    view! {<Show when=move ||state.revealed.get()><div class="solution-text"><span>"SOLUTION"</span><code>{solution.as_str().map(str::to_owned).unwrap_or_else(||serde_json::to_string(&solution).unwrap_or_default())}</code></div></Show>}
}

fn main() {
    leptos::mount::mount_to_body(App);
}
