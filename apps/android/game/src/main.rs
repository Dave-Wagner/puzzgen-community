mod model;
use gloo_net::http::Request;
use leptos::prelude::*;
use model::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wasm_bindgen_futures::spawn_local;

const LIVE: &str = "https://puzzlegen.davewagner.org";
const SAVE: &str = "puzzgen-pocket-v1";
#[derive(Clone, Default, Serialize, Deserialize)]
struct Progress {
    completed: Vec<String>,
    xp: u32,
    #[serde(default)]
    muted: bool,
    current: Option<Game>,
}
#[derive(Clone, Copy)]
struct Ui {
    game: RwSignal<Option<Game>>,
    progress: RwSignal<Progress>,
    page: RwSignal<&'static str>,
    history: RwSignal<Vec<Game>>,
    difficulty: RwSignal<String>,
    busy: RwSignal<bool>,
    message: RwSignal<String>,
    key: RwSignal<String>,
    csrf: RwSignal<String>,
    member: RwSignal<String>,
    seed: RwSignal<String>,
    zoom: RwSignal<bool>,
}
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}
fn origin() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .filter(|s| s == LIVE)
        .unwrap_or_else(|| LIVE.into())
}
async fn call(path: &str, body: Option<Value>, key: &str, csrf: &str) -> Result<Value, String> {
    let url = format!("{}{path}", origin());
    let req = if body.is_some() {
        Request::post(&url)
    } else {
        Request::get(&url)
    };
    let req = if key.is_empty() {
        req.credentials(web_sys::RequestCredentials::SameOrigin)
    } else {
        req.credentials(web_sys::RequestCredentials::Omit)
            .header("Authorization", &format!("Bearer {key}"))
    };
    let req = if body.is_some() && key.is_empty() && !csrf.is_empty() {
        req.header("X-CSRF-Token", csrf)
    } else {
        req
    };
    let response = if let Some(body) = body {
        req.json(&body).map_err(|e| e.to_string())?.send().await
    } else {
        req.send().await
    }
    .map_err(|_| {
        "Cannot reach the live API. Check your connection; practice is available without a network."
            .to_string()
    })?;
    let status = response.status();
    let retry = response.headers().get("Retry-After");
    let value: Value = response
        .json()
        .await
        .map_err(|_| format!("The API returned an unreadable response ({status})."))?;
    if !response.ok() {
        return Err(match status {401=>"Sign in using Account, return with the Games button, then refresh your session. Or connect an API key below.".into(),403=>"Permission denied. Refresh your session and check your account access.".into(),429=>format!("Your usage limit has been reached. {} No automatic retries were sent.",retry.map_or("Try again later.".into(),|s|format!("Retry after {s} seconds."))),503=>"The generator is busy. Try again in a moment.".into(),_=>value.pointer("/error/message").and_then(Value::as_str).unwrap_or("API request failed.").to_string()});
    }
    Ok(value)
}
impl Ui {
    fn save(self) {
        let game = self.game.get_untracked();
        self.progress.update(|p| p.current = game);
        if let Some(s) = storage()
            && let Ok(value) = serde_json::to_string(&self.progress.get_untracked())
        {
            let _ = s.set_item(SAVE, &value);
        }
    }
    fn sound(self, win: bool) {
        if !self.progress.get_untracked().muted
            && let Ok(a) = web_sys::HtmlAudioElement::new_with_src(if win {
                "/game/public/audio/win.ogg"
            } else {
                "/game/public/audio/click.ogg"
            })
        {
            a.set_volume(if win { 0.35 } else { 0.12 });
            let _ = a.play();
        }
    }
    fn edit(self, f: impl FnOnce(&mut Game)) {
        if self.busy.get_untracked() {
            return;
        }
        if let Some(old) = self.game.get_untracked() {
            if old.won || old.lost {
                return;
            }
            self.history.update(|h| {
                if h.len() >= 100 {
                    h.remove(0);
                }
                h.push(old);
            });
            self.game.update(|g| {
                if let Some(g) = g {
                    f(g)
                }
            });
            self.message.set(String::new());
            self.save();
            self.sound(false);
        }
    }
    fn practice(self, i: usize) {
        if self.busy.get_untracked() {
            return;
        }
        let difficulty = self.difficulty.get_untracked();
        if let Some(p) = fixtures()
            .into_iter()
            .find(|p| p.kind == KINDS[i] && p.difficulty == difficulty)
        {
            self.game.set(Some(Game::new(p, false)));
            self.history.set(vec![]);
            self.page.set("play");
            self.message.set(String::new());
            self.zoom.set(false);
            self.save();
        }
    }
    fn session(self) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        spawn_local(async move {
            match call("/api/v1/account/session", None, "", "").await {
                Ok(v) => {
                    self.csrf.set(v["csrf_token"].as_str().unwrap_or("").into());
                    self.member
                        .set(v["user"]["email"].as_str().unwrap_or("Member").into());
                    self.message
                        .set("Session connected. Choose a game and tap New live puzzle.".into());
                }
                Err(e) => {
                    self.csrf.set(String::new());
                    self.member.set(String::new());
                    self.message.set(e);
                }
            }
            self.busy.set(false);
        });
    }
    fn live(self, daily: bool) {
        if self.busy.get_untracked() {
            return;
        }
        let Some(g) = self.game.get_untracked() else {
            return;
        };
        let seed = self.seed.get_untracked();
        if !seed.is_empty() && seed.parse::<u64>().is_err() {
            self.message
                .set("Replay seed must be a whole number from 0 to 18446744073709551615.".into());
            return;
        }
        let include = g.puzzle.kind == "minesweeper";
        let mut path = if daily {
            format!(
                "/api/v1/daily/{}?difficulty={}&include_solution={include}",
                g.puzzle.kind,
                self.difficulty.get_untracked()
            )
        } else {
            format!(
                "/api/v1/puzzles/{}?difficulty={}&include_solution={include}",
                g.puzzle.kind,
                self.difficulty.get_untracked()
            )
        };
        if !daily && !seed.is_empty() {
            path.push_str(&format!("&seed={seed}"));
        }
        self.busy.set(true);
        self.message.set("Generating a fresh world…".into());
        let key = self.key.get_untracked();
        spawn_local(async move {
            match call(&path, None, &key, "").await.and_then(|v| {
                serde_json::from_value::<Puzzle>(if daily { v["puzzle"].clone() } else { v })
                    .map_err(|_| "Unexpected puzzle format.".into())
            }) {
                Ok(p) => {
                    self.game.set(Some(Game::new(p, true)));
                    self.history.set(vec![]);
                    self.message.set(
                        if daily {
                            "Today's UTC challenge is ready."
                        } else {
                            "Fresh from the live API. Have fun!"
                        }
                        .into(),
                    );
                    self.save();
                }
                Err(e) => self.message.set(e),
            }
            self.busy.set(false);
        });
    }
    fn award(self) {
        self.game.update(|g| {
            if let Some(g) = g {
                g.won = true;
            }
        });
        if let Some(g) = self.game.get_untracked() {
            let id = format!(
                "{}:{}:{}",
                g.puzzle.kind, g.puzzle.seed, g.puzzle.difficulty
            );
            self.progress.update(|p| {
                if !p.completed.contains(&id) {
                    p.completed.push(id);
                    p.xp += if g.assisted { 25 } else { 100 };
                }
            });
            self.message.set(
                if g.live {
                    "Solution accepted by the live API!"
                } else {
                    "Practice puzzle solved! Try a fresh live puzzle next."
                }
                .into(),
            );
            self.save();
            self.sound(true);
        }
    }
    fn check(self) {
        if self.busy.get_untracked() {
            return;
        }
        let Some(g) = self.game.get_untracked() else {
            return;
        };
        if g.won || g.lost {
            return;
        }
        if !g.live {
            if g.practice_correct() {
                self.award();
            } else {
                self.message
                    .set("Not quite yet. Keep exploring, or try a hint.".into());
            }
            return;
        }
        let Ok(seed) = g.puzzle.seed.parse::<u64>() else {
            self.message.set("Invalid replay seed from API.".into());
            return;
        };
        let body = json!({"kind":g.puzzle.kind,"seed":seed,"difficulty":g.puzzle.difficulty,"answer":g.answer()});
        self.busy.set(true);
        let key = self.key.get_untracked();
        let csrf = self.csrf.get_untracked();
        spawn_local(async move {
            match call("/api/v1/verify", Some(body), &key, &csrf).await {
                Ok(v) => {
                    if v["correct"] == true {
                        self.award();
                    } else {
                        self.message
                            .set("Not quite yet. The API checked your answer; keep going!".into());
                    }
                }
                Err(e) => self.message.set(e),
            }
            self.busy.set(false);
        });
    }
    fn hint(self) {
        if self.busy.get_untracked() {
            return;
        }
        let Some(g) = self.game.get_untracked() else {
            return;
        };
        if g.puzzle.solution.is_some() {
            self.edit(|g| {
                g.hint();
            });
            self.message.set("Hint applied. Assisted clears earn 25 XP. Sliding hints rewind your last move, or show the first move from the starting board.".into());
            return;
        }
        self.busy.set(true);
        let key = self.key.get_untracked();
        let path = format!(
            "/api/v1/puzzles/{}?seed={}&difficulty={}&include_solution=true",
            g.puzzle.kind, g.puzzle.seed, g.puzzle.difficulty
        );
        spawn_local(async move {
            match call(&path, None, &key, "").await {
                Ok(v) => {
                    self.busy.set(false);
                    self.edit(|g| {
                        g.puzzle.solution = Some(v["solution"].clone());
                        g.hint();
                    });
                    self.message.set("Hint applied. This fetched one puzzle from the API and marks this clear as assisted.".into());
                }
                Err(e) => {
                    self.message.set(e);
                    self.busy.set(false);
                }
            }
        });
    }
}

#[component]
fn App() -> impl IntoView {
    let saved = storage()
        .and_then(|s| s.get_item(SAVE).ok().flatten())
        .and_then(|s| serde_json::from_str::<Progress>(&s).ok())
        .unwrap_or_default();
    let ui = Ui {
        game: RwSignal::new(saved.current.clone()),
        progress: RwSignal::new(saved),
        page: RwSignal::new("home"),
        history: RwSignal::new(vec![]),
        difficulty: RwSignal::new("easy".into()),
        busy: RwSignal::new(false),
        message: RwSignal::new(String::new()),
        key: RwSignal::new(String::new()),
        csrf: RwSignal::new(String::new()),
        member: RwSignal::new(String::new()),
        seed: RwSignal::new(String::new()),
        zoom: RwSignal::new(false),
    };
    spawn_local(async move {
        if let Ok(v) = call("/api/v1/account/session", None, "", "").await {
            ui.csrf.set(v["csrf_token"].as_str().unwrap_or("").into());
            ui.member
                .set(v["user"]["email"].as_str().unwrap_or("Member").into());
        }
    });
    view! {
        <div id="pocket-worlds" class=move ||ui.game.get().filter(|_|ui.page.get()=="play").map_or("app home".to_string(),|g|format!("app theme-{}",g.puzzle.kind))>
            <header><button class="brand" on:click=move |_|{ui.page.set("home");ui.message.set(String::new());}><span class="brand-mark">"p"</span><span>"puzzgen"<small>"POCKET WORLDS"</small></span></button><div class="header-actions"><span class="xp">{move ||format!("✦ {} XP",ui.progress.get().xp)}</span><button class="icon-button" aria-label="Toggle sound" aria-pressed=move ||!ui.progress.get().muted on:click=move |_|{ui.progress.update(|p|p.muted = !p.muted);ui.save();}> {move ||if ui.progress.get().muted {"♫ off"} else {"♫ on"}}</button></div></header>
            <nav aria-label="Main navigation"><button class:active=move ||ui.page.get()=="home" on:click=move |_|ui.page.set("home")>"Explore"</button><button class:active=move ||ui.page.get()=="connect" on:click=move |_|ui.page.set("connect")>"Connection"</button><button class:active=move ||ui.page.get()=="credits" on:click=move |_|ui.page.set("credits")>"Credits"</button></nav>
            <main>
            {move ||match ui.page.get() {
                "play"=>view!{<Play ui/>}.into_any(),
                "connect"=>view!{<section class="panel connection"><span class="eyebrow">"YOUR GATEWAY"</span><h1>"A little connection."<br/>"Endless possibilities."</h1><p>"New puzzles and answer checks come from our live, personally hosted API. Usage limits keep this free learning resource available to everyone."</p><div class="status-line"><span class="online-dot"></span>"puzzlegen.davewagner.org"</div><h2>"Use your member account"</h2><p>"Sign in, then return using the Android Games button. Your secure session stays on this device."</p><div class="toolbar"><a class="button primary" href=format!("{LIVE}/account")>"Open account"</a><button disabled=move ||ui.busy.get() on:click=move |_|ui.session()>"Refresh session"</button></div><p class="subtle">{move ||if ui.member.get().is_empty() {"No member session connected.".into()} else {format!("Connected as {}",ui.member.get())}}</p><h2>"Or use an API key"</h2><label>"Personal key"<input type="password" autocomplete="off" placeholder="Paste a key for this session" on:input=move |e|ui.key.set(event_target_value(&e)) /></label><p class="subtle">"The key stays in memory only. Closing or reloading clears it. Never share your key or embed one in a public app."</p><button on:click=move |_|{ui.key.set(String::new());ui.page.set("home");}>"Continue to games"</button><h2>"Play anywhere"</h2><p>"All ten games include three fixed practice boards, one per difficulty. Practice works offline and does not consume API quota. Live mode never silently falls back to practice."</p><p class="subtle">"New live boards, daily boards, live hints, and answer checks each use your API allowance. No polling or automatic retries."</p></section>}.into_any(),
                "credits"=>view!{<Credits/>}.into_any(),
                _=>view!{<section class="hero"><div><span class="eyebrow">"TEN WORLDS. ONE CURIOUS MIND."</span><h1>"Small puzzles."<br/>"Big adventures."</h1><p>"Find your flow, follow a clue, flip a switch."<br/>"There’s a whole new world in every little challenge."</p><div class="hero-tags"><span>"10 ways to play"</span><span>"Made for curiosity"</span><span>"Free to explore"</span></div></div><div class="hero-art" aria-hidden="true"><span class="orbit o1"></span><span class="orbit o2"></span><div class="floating-tile t1">"9"</div><div class="floating-tile t2">"✦"</div><div class="floating-tile t3">"A"</div><img class="hero-spark" src="/game/public/art/star.png" alt=""/></div></section>
                <section class="collection"><div class="section-title"><div><span class="eyebrow">"PICK YOUR NEXT ADVENTURE"</span><h2>"The puzzle collection"</h2></div><span class="subtle">{move ||format!("{} unique clears",ui.progress.get().completed.len())}</span></div>
                    {move ||ui.game.get().map(|g|view!{<button class="resume" on:click=move |_|ui.page.set("play")><span>"↗ Resume "{NAMES[g.index()]}<small>{format!("{} · {} · seed {}",if g.live {"Live API"} else {"Practice"},g.puzzle.difficulty,g.puzzle.seed)}</small></span><span>"Continue →"</span></button>})}
                    <div class="cards">{(0..10).map(|i|view!{<button class=format!("game-card theme-{}",KINDS[i]) data-kind=KINDS[i] on:click=move |_|ui.practice(i)><div class="card-art" aria-hidden="true"><span class="card-orbit"></span><span class="card-glyph">{ICONS[i]}</span><span class="card-mini">{match i {0=>"1  9  3",1=>"2  4",2=>"⌜  ▣  ⌟",3=>"▪ ▫ ▪",4=>"F I N D",5=>"1  2  ◇",6=>"● ○ ●",7=>"▧ ⚑ ▧",8=>"7 × 8",_=>"A → Z"}}</span></div><div class="card-copy"><span class="eyebrow">{NAMES[i]}</span><h3>{TITLES[i]}</h3><p>{match i {0=>"A quiet corner for clear thinking.",1=>"Little numbers. Lovely discoveries.",2=>"A winding path into the unknown.",3=>"Make a picture, one square at a time.",4=>"Hidden words, waiting to be found.",5=>"Ancient tiles. A satisfying shuffle.",6=>"One last light in the deep of space.",7=>"Think carefully. Tread lightly.",8=>"Keep the gears of your mind turning.",_=>"Every letter holds a little secret."}}</p><span class="card-link">"Enter world"<span>"↗"</span></span></div></button>}).collect_view()}</div>
                </section><section class="mission panel"><span class="eyebrow">"BUILT TO SHARE"</span><h2>"Play it. Learn it. Make it yours."</h2><p>"Puzzgen is a free learning resource for people building their first games. Every live board starts with a seed, so a tiny number can become a whole new challenge."</p><a href=LIVE>"Explore the API & learning guides ↗"</a></section>}.into_any()
            }}
            <div class="notice" role="status" aria-live="polite" hidden=move ||ui.message.get().is_empty()>{move ||ui.message.get()}</div>
            </main><footer><span>"Made for the joy of figuring things out."</span><span><a href="https://www.rust-lang.org/">"Rust"</a>" · "<a href="https://leptos.dev/">"Leptos"</a>" · "<button on:click=move |_|ui.page.set("credits")>"Credits & licenses"</button></span></footer>
        </div>
    }
}

#[component]
fn Play(ui: Ui) -> impl IntoView {
    view! { {move ||ui.game.get().map(|g| {
        let i=g.index();let kind=g.puzzle.kind.clone();
        view!{<section class="play-heading"><button class="back-link" on:click=move |_|ui.page.set("home")>"← All worlds"</button><div class="world-title"><span class="world-icon">{ICONS[i]}</span><div><span class="eyebrow">{NAMES[i]}</span><h1>{TITLES[i]}</h1></div></div><div class="game-meta"><span class=if g.live {"source live"} else {"source"}>{if g.live {"● LIVE API"} else {"◌ OFFLINE PRACTICE"}}</span><span>{format!("{} · seed {}",g.puzzle.difficulty,g.puzzle.seed)}</span><span>{format!("{} moves{}",g.actions,if g.assisted {" · assisted"} else {""})}</span></div></section>
        <div class="play-layout"><section class="board-panel"><div class="board-top"><span>{match i {0|1=>"Every number has a place.",2=>"Follow your own light.",3=>"A picture is taking shape.",4=>"Look a little closer.",5=>"Put the past back in order.",6=>"Bring the stars to rest.",7=>"A careful kind of adventure.",8=>"Let’s get those gears turning.",_=>"The message is in your hands."}}</span><button class="text-button" on:click=move |_|ui.zoom.update(|v|*v = !*v)>"Zoom ↔"</button></div><div class=move ||if ui.zoom.get() {"board-scroll zoomed"} else {"board-scroll"}><Board ui/></div><Controls ui/>
        {g.won.then(||view!{<div class="result win" role="status"><img src="/game/public/art/star.png" alt=""/><h2>"Wonderfully solved!"</h2><p>{if g.assisted {"An assisted clear · 25 XP on your first clear of this board."} else {"A brilliant clear · 100 XP on your first clear of this board."}}</p><button class="primary" disabled=move ||ui.busy.get() on:click=move |_|ui.live(false)>"Another adventure →"</button></div>})}
        {g.lost.then(||view!{<div class="result loss" role="status"><h2>"A little too close!"</h2><p>"You found a mine. Take a breath and try again."</p></div>})}
        <div class="toolbar game-actions"><button class="primary" disabled=move ||ui.busy.get() || ui.game.get().is_none_or(|g|g.won||g.lost) on:click=move |_|ui.check()> {move ||if ui.busy.get() {"Working…"} else {"Check solution"}}</button><button disabled=move ||ui.busy.get() || ui.history.get().is_empty() || ui.game.get().is_none_or(|g|g.won||g.lost) on:click=move |_|{if let Some(old)=ui.history.write().pop(){ui.game.set(Some(old));ui.save();ui.message.set(String::new());}}>"Undo"</button><button disabled=move ||ui.busy.get() || ui.game.get().is_none_or(|g|g.won||g.lost) on:click=move |_|ui.hint()>"Hint ✧"</button><button disabled=move ||ui.busy.get() on:click=move |_|{if let Some(g)=ui.game.get_untracked(){ui.game.set(Some(Game::new(g.puzzle,g.live)));ui.history.set(vec![]);ui.save();}}>"Restart"</button></div></section>
        <aside><section class="panel how-to"><span class="eyebrow">"HOW TO PLAY"</span><h2>{NAMES[i]}</h2><p>{HELP[i]}</p><p class="subtle">"Tap Check solution when you’re ready. Hints mark a clear as assisted. Your XP and current board are saved on this device."</p>{(i==7).then(||view!{<p class="subtle">"This is a client-owned game: mine positions are downloaded for local reveals. It is a showcase, not an anti-cheat system."</p>})}</section><section class="panel new-world"><span class="eyebrow">"MAKE IT YOURS"</span><h2>"A fresh challenge"</h2><label>"Difficulty"<select prop:value=move ||ui.difficulty.get() disabled=move ||ui.busy.get() on:change=move |e|ui.difficulty.set(event_target_value(&e))><option value="easy">"Easy"</option><option value="medium">"Medium"</option><option value="hard">"Hard"</option></select></label><label>"Replay seed (optional)"<input inputmode="numeric" placeholder="Random when blank" prop:value=move ||ui.seed.get() on:input=move |e|ui.seed.set(event_target_value(&e)) /></label><button class="primary" disabled=move ||ui.busy.get() on:click=move |_|ui.live(false)>"New live puzzle ↗"</button><button disabled=move ||ui.busy.get() on:click=move |_|ui.live(true)>"Daily UTC challenge"</button><button disabled=move ||ui.busy.get() on:click=move |_|ui.practice(i)>"Load practice board"</button><p class="subtle">"Practice: one fixed board per difficulty. Live boards require a member session or API key. Loading a board replaces your current game."</p><button class="text-button" on:click=move |_|ui.page.set("connect")>"Manage connection →"</button></section><section class="principle"><span class="eyebrow">"THE IDEA INSIDE"</span><p>{match kind.as_str() {"sudoku"|"mini-sudoku"=>"Constraints turn a blank grid into a single solution. Our generator removes clues while preserving uniqueness.","maze"=>"A maze is a graph. Each corridor is an edge, and a path is a walk through connected cells.","nonogram"=>"Run-length clues compress a picture into numbers. Constraints bring it back into focus.","word-search"=>"A word is a vector: a start point, a direction, and a length.","sliding-puzzle"=>"Legal moves preserve solvability. A shuffled board can be replayed from its seed.","lights-out"=>"Each switch is a binary operation. Pressing twice cancels; order does not matter.","minesweeper"=>"Local counts give partial information. Deduction helps, but a random board may still require a guess.","arithmetic"=>"An expression is a tiny syntax tree. Precedence tells us which branch to solve first.",_=>"A substitution is a one-to-one mapping. Repeated letters and word shapes reveal its structure."}}</p></section></aside></div>}
    })} }
}

#[component]
fn Board(ui: Ui) -> impl IntoView {
    move || {
        ui.game.get().map(|g| {
        let kind=g.puzzle.kind.as_str();
        if kind=="arithmetic" {return view!{<div class="arithmetic-board"><div class="gear" aria-hidden="true">"⚙"</div><span class="eyebrow">"SOLVE THE MACHINE"</span><p class="expression">{g.puzzle.data["expression"].as_str().unwrap_or("").to_string()}</p><div class="answer-display" aria-label="Your answer">{if g.text.is_empty() {"?".into()} else {g.text}}</div></div>}.into_any();}
        if kind=="cryptogram" {
            let letters=g.cipher_letters();
            let words: Vec<String>=g.puzzle.data["ciphertext"].as_str().unwrap_or("").split(' ').map(String::from).collect();
            return view!{<div class="cipher-board"><span class="cipher-flourish" aria-hidden="true">"✦ ── ✧ ── ✦"</span><div class="cipher-words">{words.into_iter().map(|word|view!{<div class="cipher-word">{word.chars().map(|c|{let index=letters.iter().position(|l|*l==c);let plain=g.mapping.iter().find(|(k,_)|*k==c).map_or('·',|(_,p)|*p);view!{<button class:selected=index.is_some_and(|i|g.selected==Some([i,0])) disabled=index.is_none() aria-label=format!("Cipher letter {c}") on:click=move |_|if let Some(i)=index {ui.edit(|g|g.selected=Some([i,0]));}><strong>{if c.is_ascii_alphabetic() {plain} else {c}}</strong><small>{c}</small></button>}}).collect_view()}</div>}).collect_view()}</div><p>"Select a symbol. Reveal its letter."</p></div>}.into_any();
        }
        let rows=g.rows();let cols=g.cols();let nonogram=kind=="nonogram";
        let board_class=format!("puzzle-grid {} {}",kind,if nonogram {"with-clues"} else {""});
        let style=format!("--cols:{};--rows:{rows};",cols+usize::from(nonogram));
        let mut cells=Vec::new();
        if nonogram {cells.push(view!{<span class="clue corner">"↓ →"</span>}.into_any());for c in 0..cols {let clue=g.puzzle.data["column_clues"][c].as_array().map(|a|a.iter().map(Value::to_string).collect::<Vec<_>>().join(" ")).unwrap_or_default();cells.push(view!{<span class="clue column-clue">{clue}</span>}.into_any());}}
        for r in 0..rows {
            if nonogram {let clue=g.puzzle.data["row_clues"][r].as_array().map(|a|a.iter().map(Value::to_string).collect::<Vec<_>>().join(" ")).unwrap_or_default();cells.push(view!{<span class="clue row-clue">{clue}</span>}.into_any());}
            for c in 0..cols {
                let p=[r,c];let value=g.grid[r][c];let mut class="cell".to_string();let mut label=String::new();let mut disabled=g.won||g.lost;
                if g.selected==Some(p) {class.push_str(" selected");}
                match kind {
                    "sudoku"|"mini-sudoku"=>{let size=g.puzzle.data["box_size"].as_u64().unwrap_or(3) as usize;if g.puzzle.data["grid"][r][c]!=0 {class.push_str(" given");} else {class.push_str(" editable");}
if c%size==0 {class.push_str(" box-left");}
if r%size==0 {class.push_str(" box-top");}
if value>0 {label=value.to_string();let conflict=(0..cols).any(|x|x!=c && g.grid[r][x]==value)||(0..rows).any(|x|x!=r && g.grid[x][c]==value)||(r/size*size..r/size*size+size).any(|rr|(c/size*size..c/size*size+size).any(|cc|[rr,cc]!=p && g.grid[rr][cc]==value));if conflict {class.push_str(" conflict");}}}
                    "maze"=>{if value==1 {class.push_str(" wall");disabled=true;}
else {class.push_str(" corridor");}
if g.path.contains(&p) {class.push_str(" trail");}
if g.path.last()==Some(&p) {class.push_str(" explorer");label="✦".into();}
else if json!(p)==g.puzzle.data["end"] {label="⚑".into();class.push_str(" exit");}}
                    "nonogram"=>{if value==1 {class.push_str(" painted");}
if value==-1 {label="×".into();class.push_str(" crossed");}}
                    "word-search"=>{label=g.puzzle.data["grid"][r][c].as_str().unwrap_or("").into();if g.found.iter().any(|w|coords(&w["cells"]).contains(&p)) {class.push_str(" found");}}
                    "sliding-puzzle"=>{if value==0 {class.push_str(" empty");}
else {label=value.to_string();class.push_str(" stone");if value==(r*cols+c+1) as i32 {class.push_str(" placed");}}}
                    "lights-out"=>{class.push_str(if value==1 {" lit"} else {" unlit"});label=if value==1 {"●"} else {"○"}.into();}
                    "minesweeper"=>{if g.revealed.contains(&p) {class.push_str(" revealed");if g.mine(p) {label="✹".into();class.push_str(" mine");}
else {let n=g.adjacent(p);if n>0 {label=n.to_string();class.push_str(&format!(" count-{n}"));}}}else if g.flags.contains(&p) {label="⚑".into();class.push_str(" flagged");}
else {class.push_str(" covered");if g.lost && g.mine(p) {label="✹".into();}}}
                    _=>{}
                }
                let aria=format!("Row {}, column {}{}",r+1,c+1,if label.is_empty() {String::new()} else {format!(", {label}")});
                cells.push(view!{<button class=class aria-label=aria data-row=r data-col=c disabled=disabled on:click=move |_|ui.edit(|g|g.tap(p))>{label}</button>}.into_any());
            }
        }
        view!{<div class=board_class style=style>{cells}</div>}.into_any()
    })
    }
}

#[component]
fn Controls(ui: Ui) -> impl IntoView {
    move || {
        ui.game.get().map(|g|match g.puzzle.kind.as_str() {
        "sudoku"|"mini-sudoku"=>{let n=g.cols();view!{<div class="number-pad">{(1..=n).map(|n|view!{<button aria-label=format!("Enter {n}") on:click=move |_|ui.edit(|g|g.enter(&n.to_string()))>{n}</button>}).collect_view()}<button on:click=move |_|ui.edit(|g|g.enter("0"))>"Erase"</button></div>}.into_any()}
        "arithmetic"=>view!{<div class="math-pad">{["1","2","3","4","5","6","7","8","9","-","0","erase"].into_iter().map(|s|view!{<button on:click=move |_|ui.edit(|g|g.enter(s))>{if s=="erase" {"⌫"} else {s}}</button>}).collect_view()}</div>}.into_any(),
        "cryptogram"=>view!{<div class="letter-pad">{('A'..='Z').map(|c|{let used=g.mapping.iter().any(|(_,p)|*p==c);view!{<button class:used=used on:click=move |_|ui.edit(|g|g.enter(&c.to_string()))>{c}</button>}}).collect_view()}<button class="erase-letter" on:click=move |_|ui.edit(|g|g.enter(""))>"Erase"</button></div>}.into_any(),
        "maze"=>view!{<div class="dpad">{[("up","↑"),("left","←"),("down","↓"),("right","→")].into_iter().map(|(d,s)|view!{<button class=d aria-label=format!("Move {d}") on:click=move |_|ui.edit(|g|g.step(d))>{s}</button>}).collect_view()}</div>}.into_any(),
        "nonogram"|"minesweeper"=>{let mines=g.puzzle.kind=="minesweeper";view!{<div class="mode-controls"><button class:active=!g.alternate aria-pressed=!g.alternate on:click=move |_|ui.edit(|g|g.alternate=false)>{if mines {"Reveal"} else {"Paint"}}</button><button class:active=g.alternate aria-pressed=g.alternate on:click=move |_|ui.edit(|g|g.alternate=true)>{if mines {"⚑ Flag"} else {"× Cross"}}</button><span>{if mines {format!("{} / {} flags",g.flags.len(),g.puzzle.data["mine_count"])}else {"Crosses are notes, not painted cells.".into()}}</span></div>}.into_any()}
        "word-search"=>view!{<div class="word-list">{g.puzzle.data["words"].as_array().cloned().unwrap_or_default().into_iter().map(|w|{let found=g.found.iter().any(|f|f["word"]==w);view!{<span class:found=found>{format!("{}{}",if found {"✓ "}else {""},w.as_str().unwrap_or(""))}</span>}}).collect_view()}</div><p class="control-note">{if g.selected.is_some() {"Now tap the last letter. Tap the same square to cancel."} else {"Tap the first letter, then the last."}}</p>}.into_any(),
        _=>view!{<p class="control-note">"Tap a tile beside the empty space to move it."</p>}.into_any()
    })
    }
}

#[component]
fn Credits() -> impl IntoView {
    let manifest: Value =
        serde_json::from_str(include_str!("../public/credits.json")).unwrap_or_default();
    let packs = manifest["packs"].as_array().cloned().unwrap_or_default();
    view! {<section class="panel credits"><span class="eyebrow">"GOOD THINGS ARE MADE TOGETHER"</span><h1>"Credits & contributors"</h1><p>"Puzzgen is personally hosted by Dave Wagner as a free resource for curious players and people learning to build games. Thank you to the creators who make their work available for everyone."</p><h2>"Art & audio · Kenney"</h2><p>"These packs are included under Creative Commons Zero (CC0 1.0). We credit Kenney even though attribution is not required. Individual source notices and a file manifest are bundled with the app."</p><div class="credit-packs">{packs.into_iter().map(|p|{let id=p["id"].as_str().unwrap_or("").to_string();view!{<article><h3>{p["name"].as_str().unwrap_or("").to_string()}</h3><p>{p["usage"].as_str().unwrap_or("").to_string()}</p><a href=p["source"].as_str().unwrap_or("").to_string()>"Source pack ↗"</a>" · "<a href=format!("/game/public/licenses/{id}.txt")>"Original license"</a></article>}}).collect_view()}</div><p><a href="https://creativecommons.org/publicdomain/zero/1.0/">"Read CC0 1.0 ↗"</a>" · "<a href="/game/public/credits.json">"Bundled file and SHA-256 manifest"</a></p><h2>"Game design & technology"</h2><p>"Original layouts, themes, CSS illustration and game interaction code by the Puzzgen project. Puzzle generation and validation use the Puzzgen Rust API. Interface built with Rust, Leptos and WebAssembly; packaged in an Android WebView. No artwork is presented as original Puzzgen art."</p><p>"Progress and XP are local to your device. Practice boards are a finite set generated from seed 42. Live generation is deterministic: the same puzzle type, difficulty, seed and generator version reproduce a board. Puzzle variation is large, not mathematically infinite unique content."</p><h2>"Contribute a little wonder"</h2><p>"Ideas, accessible interactions, new puzzle families, tutorials and translations are welcome. If future packs require attribution, their author, license, source and modification notes belong here before shipping."</p><p><a href="https://github.com/Dave-Wagner/puzzgen">"Project on GitHub ↗"</a>" · "<a href=LIVE>"API & learning resource ↗"</a></p></section>}
}

fn main() {
    leptos::mount::mount_to_body(App);
}
