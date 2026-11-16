use super::*;

#[component]
pub(super) fn LanguageBoard(puzzle: Puzzle, state: Workbench) -> impl IntoView {
    let kind = puzzle.kind.clone();
    let ladder = kind == "word-ladder";
    let heading = match kind.as_str() {
        "spoonerism" => "SWAP THE SOUNDS",
        "anagram" => "REARRANGE THE LETTERS",
        "word-ladder" => "ONE LETTER AT A TIME",
        _ => "FIND THE CONNECTING WORD",
    };
    let data = &puzzle.data;
    let prompt = match kind.as_str() {
        "spoonerism" => data["source_phrase"].as_str().unwrap_or("").to_owned(),
        "anagram" => data["letters"]
            .as_str()
            .unwrap_or("")
            .chars()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" · "),
        "word-ladder" => format!(
            "{} → {}",
            data["start"].as_str().unwrap_or(""),
            data["end"].as_str().unwrap_or("")
        ),
        _ => format!(
            "{} + ? + {}",
            data["left"].as_str().unwrap_or(""),
            data["right"].as_str().unwrap_or("")
        ),
    };
    let clue = data["clue"]
        .as_str()
        .unwrap_or("Use the supplied vocabulary to find an answer.")
        .to_owned();
    let hint = data["hint"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| {
            data["first_letter"]
                .as_str()
                .map(|s| format!("First letter: {s}"))
        })
        .unwrap_or_default();
    let length = data["length"]
        .as_u64()
        .map(|n| format!("{n} letters"))
        .or_else(|| {
            data["word_lengths"].as_array().map(|v| {
                format!(
                    "Word lengths: {}",
                    v.iter()
                        .map(Value::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
        })
        .unwrap_or_default();
    let steps = if ladder {
        format!(
            "Shortest path: {} changes. You may use up to {} changes. Include both endpoints; do not repeat words.",
            data["minimum_steps"], data["max_steps"]
        )
    } else {
        length
    };
    let words = data["choices"]
        .as_array()
        .or_else(|| data["allowed_words"].as_array())
        .cloned()
        .unwrap_or_default();
    let lesson = format!("/lessons/{kind}.html");
    let objective = data["learning"]["objective"]
        .as_str()
        .unwrap_or("Explore the pattern behind the words")
        .to_owned();
    let principle = data["learning"]["principle"]
        .as_str()
        .unwrap_or("Use the clue and notice how each word changes.")
        .to_owned();
    let experiment = data["learning"]["experiment"]
        .as_str()
        .unwrap_or("Explain the rule to someone else after solving.")
        .to_owned();
    let source = prompt.clone();
    let left = data["left"].as_str().unwrap_or("").to_owned();
    let right = data["right"].as_str().unwrap_or("").to_owned();
    let solution = puzzle.solution.unwrap_or(Value::Null);
    let sounds = data["source_sounds"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|s| format!("/{s}/"))
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_default();
    let worked_solution = solution.clone();
    let worked = Memo::new(move |_| {
        let answer = if state.revealed.get() {
            worked_solution
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    worked_solution
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_default()
                })
        } else {
            state.answer.get()
        };
        match kind.as_str() {
            "spoonerism" => format!(
                "{source} → {}. The opening consonant sounds exchange places; the remaining sounds stay in order. Say both phrases slowly to hear the swap.",
                answer.trim()
            ),
            "anagram" => {
                let mut letters = answer
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_ascii_alphabetic())
                    .collect::<Vec<_>>();
                letters.sort_unstable();
                format!(
                    "{} uses the same letter counts as the puzzle. Sorted letters: {}. Repeated letters must be counted separately.",
                    answer.trim(),
                    letters
                        .iter()
                        .map(char::to_string)
                        .collect::<Vec<_>>()
                        .join(" · ")
                )
            }
            "compound-bridge" => format!(
                "{left} + {0} forms {left}{0}; {0} + {right} forms {0}{right}. Compounds may conventionally use spaces or hyphens: meaning matters as well as spelling.",
                answer.trim()
            ),
            _ => {
                let words = answer.split_whitespace().collect::<Vec<_>>();
                format!(
                    "{}: {} words, {} one-letter changes. A word is a vertex and a legal change is an edge. Your route need not match the revealed shortest route.",
                    words.join(" → "),
                    words.len(),
                    words.len().saturating_sub(1)
                )
            }
        }
    });
    view! {
        <div class="board-center language-board">
            <aside class="language-learning" aria-label="Learning goal">
                <span class="section-label">"TODAY'S DISCOVERY"</span>
                <h3>{objective}</h3><p>{principle}</p>
                <p class="exercise">{experiment}</p>
            </aside>
            <span class="text-puzzle-label">{heading}</span>
            <h3 class="language-prompt">{prompt}</h3>
            <p class="language-clue">{clue}</p><p class="board-caption">{steps}</p>
            {(!hint.is_empty()).then(||view!{<p class="language-hint">{hint}</p>})}
            {(!sounds.is_empty()).then(||view!{<p class="board-caption">"Sound guide (US English): "{sounds}</p>})}
            <div class="language-choices" role="group" aria-label=if ladder{"Allowed words: select to append to your path"}else{"Answer options"}>
                {words.into_iter().filter_map(|v|v.as_str().map(str::to_owned)).map(|word|{
                    let value=word.clone();
                    view!{<button class="recipe-tab" disabled=move ||state.busy.get()||state.revealed.get() on:click=move |_|{
                        if ladder {state.answer.update(|a|{if !a.is_empty(){a.push(' ');}a.push_str(&value);});}else{state.answer.set(value.clone());}
                        state.result.set(String::new());
                    }>{word}</button>}
                }).collect_view()}
            </div>
            {ladder.then(||view!{<div class="language-path"><p role="status">{move ||{let a=state.answer.get();if a.is_empty(){"Your path will appear here.".into()}else{a.split_whitespace().collect::<Vec<_>>().join(" → ")}}}</p>
                <button class="text-button" disabled=move ||state.busy.get()||state.revealed.get() on:click=move |_|{state.answer.update(|a|{let mut words=a.split_whitespace().collect::<Vec<_>>();words.pop();*a=words.join(" ");});state.result.set(String::new());}>"Undo word"</button>
                <button class="text-button" disabled=move ||state.busy.get()||state.revealed.get() on:click=move |_|{state.answer.set(String::new());state.result.set(String::new());}>"Clear path"</button>
            </div>})}
            <SolutionText solution=solution state=state/>
            <Show when=move || state.revealed.get() || state.result.get().starts_with("Nicely solved!")>
                <aside class="language-learning" aria-label="Worked explanation"><h3>"Why it works"</h3><p>{move || worked.get()}</p></aside>
            </Show>
            <a class="language-lesson" href=lesson>"Learn how this puzzle works →"</a>
            <small>"Original CC0 content · US English · Curated answers; some accents and alternate phrases differ."</small>
        </div>
    }
}
