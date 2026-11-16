use leptos::prelude::*;

const EXAMPLES: [(&str, &str, &str); 4] = [
    (
        "Plain JavaScript",
        "plain.js",
        include_str!("../examples/plain.js"),
    ),
    (
        "React",
        "Puzzle.jsx",
        include_str!("../examples/Puzzle.jsx"),
    ),
    ("Vue", "Puzzle.vue", include_str!("../examples/Puzzle.vue")),
    (
        "Svelte",
        "Puzzle.svelte",
        include_str!("../examples/Puzzle.svelte"),
    ),
];

#[component]
pub fn LearningHub() -> impl IntoView {
    let selected = RwSignal::new(0_usize);
    view! {
        <section id="learn" class="learning-hub" aria-labelledby="learn-title">
            <div class="eyebrow"><span class="tiny-square"></span>"LEARN SOMETHING. MAKE IT YOURS."</div>
            <h2 id="learn-title">"Your interface. Our puzzles."</h2>
            <p class="learning-intro">"Puzzgen exists to lower the first hurdle. You should be able to practice layouts, interactions, and game ideas without building an API first. This is a personally hosted, free learning resource for beginners and curious developers alike."</p>
            <div class="learning-steps">
                <article><span>"01 / EXPLORE"</span><h3>"Play with the data."</h3><p>"Choose a puzzle above. Open Under the hood. Notice how a grid becomes a JSON array, and how the same seed gives you the same challenge."</p></article>
                <article><span>"02 / BUILD"</span><h3>"Make one small game."</h3><p>"Start with arithmetic: one expression, one input, one check. Add loading, empty, error, and success states before adding a scoreboard."</p><a href="/examples/index.html">"Run the plain JavaScript starter →"</a></article>
                <article><span>"03 / EXPERIMENT"</span><h3>"Change one thing."</h3><p>"Try a different layout. Add keyboard controls. Make a native screen. Keep the API contract and compare how different tools express the same interaction."</p></article>
            </div>

            <div class="lesson-heading"><div><span class="section-label">"START WITH A REAL REQUEST"</span><h3>"A tiny API, a lot to discover."</h3></div><a href="/docs">"Full API reference ↗"</a></div>
            <pre class="learning-code"><code>{"// Use your deployed Puzzgen origin, or localhost during development.\nconst response = await fetch(\n  `${apiOrigin}/api/v1/puzzles/arithmetic?seed=42&difficulty=easy`\n);\nconst puzzle = await response.json();\nif (!response.ok) throw new Error(puzzle.error.message);\n// Render puzzle.data.expression as text. Never evaluate server text as code."}</code></pre>
            <p class="learning-note">"On member instances, create a free account and a personal API key. The playground can use your signed-in session. Cross-origin API-key requests are supported. The current instance budget is shared: request on user actions, reuse the puzzle you have, and respect 429/503 plus Retry-After. Hosts can change limits or enable keys; see the API reference. Never embed a private server key in a public app."</p>

            <div class="lesson-heading"><div><span class="section-label">"SAME IDEA / DIFFERENT TOOLS"</span><h3>"Start plain. Find your framework."</h3></div></div>
            <div class="recipe-tabs" role="group" aria-label="Framework examples">{EXAMPLES.into_iter().enumerate().map(|(index,(name,_,_))| view! {
                <button class=move || if selected.get()==index {"recipe-tab active"} else {"recipe-tab"} aria-pressed=move || (selected.get()==index).to_string() on:click=move |_| selected.set(index)>{name}</button>
            }).collect_view()}</div>
            <div class="recipe-description"><p>{move || if selected.get()==0 {"Complete, buildless starter: generate and verify an answer, with loading and error states. Open it, inspect it, and remix the interface."}else{"Component recipe for an existing app: load an expression with loading and error states. It is a starting point, not a complete framework project; add answer checking using the plain starter as your guide."}}</p><a href=move || format!("/examples/{}", EXAMPLES[selected.get()].1)>"Open source file ↗"</a></div>
            <pre class="learning-code recipe-code"><code>{move || EXAMPLES[selected.get()].2}</code></pre>
            <p class="learning-note">"Try the "<a href="/examples/index.html?offline">"offline practice mode"</a>" to work on the interface with no API calls. Framework recipes are source examples, not separately installed and tested framework applications. Use an HTTPS API origin when publishing. For older browsers, replace AbortSignal.timeout with an AbortController plus a timer."</p>

            <div class="lesson-heading"><div><span class="section-label">"THE IDEAS UNDER THE INTERFACE"</span><h3>"A small field guide."</h3></div></div>
            <div class="principle-list">
                {[
                    ("Seeds & reproducibility", "A seed starts a deterministic pseudo-random sequence. Keep kind, seed, difficulty, and generator version together to replay a puzzle. Many seeds can lead to the same puzzle; a finite seed space is not literally infinite. Try seed 42 twice, then change only the difficulty.", "Exercise: put the seed in a share link and restore the screen from that link."),
                    ("Generating is not solving", "A generator constructs the challenge. A solver finds an answer. A validator checks your proposed answer. Sliding puzzles are built through legal moves so they remain reachable; Sudoku removes clues only when a bounded solver can still establish uniqueness.", "Exercise: animate one legal sliding move before implementing a complete board."),
                    ("Constraints & uniqueness", "A valid puzzle may have several answers. Sudoku and nonograms here are checked for uniqueness. Lights Out accepts different press patterns that solve the board. Minesweeper guarantees a safe opening, not a no-guess solution. Read each response's uniqueness field.", "Exercise: explain why comparing your answer with one saved solution can reject another valid answer."),
                    ("Grids & coordinates", "A grid is an array of rows. Use grid[row][column]; API coordinates start at zero. Keep the model separate from how the board looks, and use stable row/column identities when rendering cells.", "Exercise: render a 4×4 board as buttons, then add descriptive labels and keyboard navigation."),
                    ("UI state & async requests", "Model idle, loading, ready, checking, success, and error explicitly. Disable duplicate submissions. Keep the current puzzle and its answer together, and discard responses belonging to an older request. Do not show a stale solution after loading a new puzzle.", "Exercise: throttle your network and check that repeated clicks cannot mix two puzzles."),
                    ("Replayable tests & offline fixtures", "A fixed seed is a repeatable test case. A saved JSON fixture lets you refine your interface without making more API calls. Keep fixtures free of personal data; include the generator version so changes are visible.", "Exercise: use /examples/fixtures/arithmetic.json to build an offline loading/error/success story."),
                    ("Accessible play", "Use buttons for actions and visible labels for inputs. Make focus visible, announce results politely, and avoid conveying state only through color. A playable mouse demo is only the beginning; test with the keyboard and a screen reader.", "Exercise: complete one round without touching the mouse."),
                    ("Trust, privacy & fair use", "Seeds and solution reveal are useful learning tools, not anti-cheat. Never treat a browser's score as authoritative for competitive play. The service is stateless today: save your own progress locally, avoid personal data in seeds, and request only what you need.", "Exercise: add a Reset progress control beside local storage, and explain what it deletes."),
                ].into_iter().map(|(title,body,exercise)| view! {<details class="principle"><summary>{title}</summary><p>{body}</p><p class="exercise">{exercise}</p></details>}).collect_view()}
            </div>

            <div class="lesson-heading"><div><span class="section-label">"LANGUAGE LAB"</span><h3>"Words have patterns, too."</h3></div></div>
            <div class="learning-steps">
                {[("spoonerism", "Spoonerisms", "Swap sounds, not letters. Learn how a curated phoneme pair becomes a clue."), ("anagram", "Anagrams", "Count repeated letters and distinguish a clue from a vocabulary constraint."), ("word-ladder", "Word ladders", "Build a graph, find shortest paths, and accept alternative solutions."), ("compound-bridge", "Compound bridges", "Connect meanings with one missing word and an honest answer policy.")].into_iter().map(|(slug,title,body)|view!{<article><h3>{title}</h3><p>{body}</p><a href=format!("/lessons/{slug}.html")>"Open the interactive workshop →"</a></article>}).collect_view()}
            </div>
            <div class="lesson-heading"><div><span class="section-label">"DISCOVERY WORKSHOPS"</span><h3>"Seven more ways to learn through play."</h3></div></div>
            <div class="learning-steps">
                {[("word-sorting", "Word sorting", "Classification and stable identities."),("sequence-detective", "Sequence detective", "Explicit assumptions and rule families."),("memory-pairs", "Memory pairs", "State machines, positions and matching."),("path-connections", "Path connections", "Bit masks, rotation and connectivity."),("futoshiki", "Futoshiki", "Latin squares, inequalities and uniqueness."),("sound-families", "Sound families", "Pronunciation beyond spelling."),("sentence-assembly", "Sentence assembly", "Controlled grammar and alternative answers.")].into_iter().map(|(slug,title,body)|view!{<article><h3>{title}</h3><p>{body}</p><a href=format!("/lessons/{slug}.html")>"Open the interactive workshop →"</a></article>}).collect_view()}
            </div>
            <div class="lesson-heading"><div><span class="section-label">"GROWING THE LIBRARY / CLEARLY LABELED"</span><h3>"Room for the next discovery."</h3></div></div>
            <div class="learning-roadmap">
                {[
                    ("Web game workshops", "Planned", "Step-by-step Sudoku, maze path input, word selection, sliding tiles, and Lights Out. Each workshop will connect data, controls, validation, and accessible feedback."),
                    ("Native patterns", "Outline", "React Native: request state plus FlatList grids. Flutter: async service plus GridView. SwiftUI and Jetpack Compose: state-driven screens. Next: HTTPS, emulator host addresses, lifecycle cancellation, and offline progress."),
                    ("Framework deep dives", "Outline", "React reducers and effects; Vue composables; Svelte reactive state; plain DOM event delegation. Compare cancellation, derived state, focus handling, and testing with the same puzzle."),
                    ("More puzzles & topics", "Ideas", "Kakuro, KenKen, Slitherlink, Hashi, logic deductions, and graph puzzles. Add a generator, validator, runtime budget, and honest guarantees before publishing an endpoint."),
                ].into_iter().map(|(title,status,outline)|view!{<article><span class="lesson-status">{status}</span><h3>{title}</h3><p>{outline}</p></article>}).collect_view()}
            </div>
            <p class="learning-note">"This library grows one useful example at a time. Planned and outline sections are not completed tutorials. You can practice with the working API and starter today; no AI service is required to generate puzzles or run your game."</p>
        </section>
    }
}
