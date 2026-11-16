use super::*;
fn answer(state: Workbench) -> Value {
    serde_json::from_str(&state.answer.get()).unwrap_or(Value::Null)
}
fn set(state: Workbench, value: Value) {
    state.answer.set(value.to_string());
    state.result.set(String::new());
}
fn selected(state: Workbench, id: &str) -> bool {
    answer(state)
        .as_array()
        .is_some_and(|v| v.iter().any(|v| v.as_str() == Some(id)))
}
fn blocked(state: Workbench) -> bool {
    state.busy.get() || state.revealed.get()
}
#[component]
pub(super) fn DiscoveryBoard(puzzle: Puzzle, state: Workbench) -> impl IntoView {
    let data = puzzle.data;
    let solution = puzzle.solution.unwrap_or(Value::Null);
    let kind = puzzle.kind;
    let objective = data["learning"]["objective"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    let principle = data["learning"]["principle"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    let lesson = format!("/lessons/{kind}.html");
    let board = match kind.as_str() {
        "word-sorting" => {
            let categories = data["categories"].as_array().cloned().unwrap_or_default();
            let items = data["items"].as_array().cloned().unwrap_or_default();
            view!{<div class="sorting-items">{items.into_iter().map(|item|{let id=item["id"].as_str().unwrap_or("").to_owned();let idvalue=id.clone();let text=item["text"].as_str().unwrap_or("").to_owned();let solved=solution[&id].as_str().unwrap_or("").to_owned();let options=categories.clone();view!{<label class="sorting-item"><strong>{text}</strong><select aria-label=format!("Category for {}",item["text"].as_str().unwrap_or("")) disabled=move||blocked(state) prop:value=move||if state.revealed.get(){solved.clone()}else{answer(state)[&idvalue].as_str().unwrap_or("").to_owned()} on:change=move|ev|{let mut map=answer(state).as_object().cloned().unwrap_or_default();let value=event_target_value(&ev);if value.is_empty(){map.remove(&id);}else{map.insert(id.clone(),json!(value));}set(state,Value::Object(map));}><option value="">"Choose a category"</option>{options.into_iter().map(|c|view!{<option value=c["id"].as_str().unwrap_or("").to_owned()>{c["label"].as_str().unwrap_or("").to_owned()}</option>}).collect_view()}</select></label>}}).collect_view()}</div>}.into_any()
        }
        "sequence-detective" => {
            let terms = data["terms"].as_array().cloned().unwrap_or_default();
            let solved = solution.to_string();
            let family = data["family"].as_str().unwrap_or("").to_owned();
            view!{<p><strong>"Rule family: "</strong>{family}</p><div class="sequence-terms">{terms.into_iter().map(|n|view!{<span>{if n.is_null(){"?".into()}else{n.to_string()}}</span>}).collect_view()}</div><label>"Missing integer"<input type="number" step="1" disabled=move||blocked(state) prop:value=move||if state.revealed.get(){solved.clone()}else{state.answer.get()} on:input=move|ev|{state.answer.set(event_target_value(&ev));state.result.set(String::new());}/></label>}.into_any()
        }
        "path-connections" | "futoshiki" => {
            let pipes = kind == "path-connections";
            let original = grid(&data["grid"]);
            let solved = grid(&solution);
            let n = original.len();
            let mut cells = vec![];
            for r in 0..n {
                for c in 0..n {
                    let given = original[r][c];
                    let completed = solved
                        .get(r)
                        .and_then(|row| row.get(c))
                        .copied()
                        .unwrap_or(0);
                    if pipes {
                        cells.push(view!{<button class="pipe-tile" disabled=move||blocked(state) aria-label=move||{let mask=if state.revealed.get(){completed}else{state.edits.get().get(r).and_then(|row|row.get(c)).copied().unwrap_or(0)};format!("Row {}, column {}. Ports: {}. Rotate clockwise",r+1,c+1,ports(mask))} on:click=move|_|{state.edits.update(|rows|{let v=&mut rows[r][c];*v=((*v<<1)&15)|(*v>>3);});state.result.set(String::new());}>{move||{let mask=if state.revealed.get(){completed}else{state.edits.get().get(r).and_then(|row|row.get(c)).copied().unwrap_or(0)};["·","╵","╶","└","╷","│","┌","├","╴","┘","─","┴","┐","┤","┬","┼"][mask as usize]}}</button>}.into_any());
                    } else {
                        cells.push(view!{<input class={if given > 0 {"latin-cell given"}else{"latin-cell"}} type="number" min="1" max="4" step="1" inputmode="numeric" aria-label=format!("Row {}, column {}{}",r+1,c+1,if given>0{", given"}else{""}) disabled=move||{given>0||blocked(state)} prop:value=move||{let v=if state.revealed.get(){completed}else{state.edits.get().get(r).and_then(|row|row.get(c)).copied().unwrap_or(0)};if v==0{String::new()}else{v.to_string()}} on:input=move|ev|{let value=event_target_value(&ev).parse::<u8>().unwrap_or(0);state.edits.update(|rows|rows[r][c]=value);state.result.set(String::new());}/>}.into_any());
                    }
                }
            }
            let inequalities = data["inequalities"].as_array().cloned().unwrap_or_default();
            view!{<p>{if pipes{"Tap a tile to rotate clockwise. Join every port, including all branches. No ports may point off the board. The top-left tile is the source."}else{"Use 1–4 once in each row and column. There are no Sudoku boxes. Follow the inequality clues below."}}</p><div class="discovery-grid" style=format!("grid-template-columns:repeat({n},minmax(0,1fr))")>{cells}</div>{(!pipes).then(||view!{<ul class="inequality-list">{inequalities.into_iter().map(|e|view!{<li>{format!("r{}c{} {} r{}c{}",e["first"][0].as_u64().unwrap_or(0)+1,e["first"][1].as_u64().unwrap_or(0)+1,e["op"].as_str().unwrap_or(""),e["second"][0].as_u64().unwrap_or(0)+1,e["second"][1].as_u64().unwrap_or(0)+1)}</li>}).collect_view()}</ul>})}}.into_any()
        }
        "sound-families" => {
            let words = data["words"].as_array().cloned().unwrap_or_default();
            let anchor = data["anchor"].as_str().unwrap_or("").to_owned();
            let hint = data["hint"].as_str().unwrap_or("").to_owned();
            view!{<h3>"What rhymes with “"{anchor}"”?"</h3><p>{hint}" · General US English"</p><div class="language-choices">{words.into_iter().map(|w|{let id=w["id"].as_str().unwrap_or("").to_owned();let active=id.clone();let solved=solution.as_array().is_some_and(|a|a.iter().any(|v|v.as_str()==Some(&id)));view!{<button class="recipe-tab" aria-pressed=move||(if state.revealed.get(){solved}else{selected(state,&active)}).to_string() disabled=move||blocked(state) on:click=move|_|{let mut ids=answer(state).as_array().cloned().unwrap_or_default();if let Some(i)=ids.iter().position(|v|v.as_str()==Some(&id)){ids.remove(i);}else{ids.push(json!(id));}set(state,json!(ids));}>{w["text"].as_str().unwrap_or("").to_owned()}</button>}}).collect_view()}</div><p>"Select all three matches. Press again to deselect."</p>}.into_any()
        }
        "sentence-assembly" => {
            let tiles = data["tiles"].as_array().cloned().unwrap_or_default();
            let for_preview = tiles.clone();
            let solved = solution.clone();
            let hint = data["hint"].as_str().unwrap_or("").to_owned();
            view!{<p>{hint}</p><div class="sentence-preview" role="status">{move||{let ids=if state.revealed.get(){solved.as_array().cloned().unwrap_or_default()}else{answer(state).as_array().cloned().unwrap_or_default()};if ids.is_empty(){"Your sentence will appear here.".into()}else{ids.iter().filter_map(|id|for_preview.iter().find(|t|t["id"]==*id).and_then(|t|t["text"].as_str())).collect::<Vec<_>>().join(" ")}}}</div><div class="language-choices">{tiles.into_iter().map(|t|{let id=t["id"].as_str().unwrap_or("").to_owned();let check=id.clone();view!{<button class="recipe-tab" disabled=move||blocked(state)||selected(state,&check) on:click=move|_|{let mut ids=answer(state).as_array().cloned().unwrap_or_default();ids.push(json!(id));set(state,json!(ids));}>{t["text"].as_str().unwrap_or("").to_owned()}</button>}}).collect_view()}</div><div class="language-choices"><button class="text-button" disabled=move||blocked(state) on:click=move|_|{let mut ids=answer(state).as_array().cloned().unwrap_or_default();ids.pop();set(state,json!(ids));}>"Undo word"</button><button class="text-button" disabled=move||blocked(state) on:click=move|_|set(state,json!([]))>"Clear sentence"</button></div>}.into_any()
        }
        _ => view! {<MemoryBoard data=data.clone() state=state/>}.into_any(),
    };
    view! {<div class="board-center language-board discovery-board"><aside class="language-learning"><span class="section-label">"TODAY'S DISCOVERY"</span><h3>{objective}</h3><p>{principle}</p></aside>{board}<a href=lesson class="language-lesson">"Explore the interactive teaching lab →"</a><small>"Original CC0 examples · Seeded construction data · Your progress stays in this page until you generate another puzzle."</small></div>}
}
fn ports(mask: u8) -> String {
    ["north", "east", "south", "west"]
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .map(|(_, s)| *s)
        .collect::<Vec<_>>()
        .join(", ")
}
#[component]
fn MemoryBoard(data: Value, state: Workbench) -> impl IntoView {
    let cards = data["cards"].as_array().cloned().unwrap_or_default();
    let open = RwSignal::new(Vec::<usize>::new());
    let initial_pairs = answer(state)
        .as_array()
        .map(|pairs| {
            pairs
                .iter()
                .filter_map(|pair| {
                    let ids = pair.as_array()?;
                    if ids.len() != 2 {
                        return None;
                    }
                    Some([
                        cards.iter().position(|c| c["id"] == ids[0])?,
                        cards.iter().position(|c| c["id"] == ids[1])?,
                    ])
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let matched = RwSignal::new(initial_pairs);
    let message = RwSignal::new(
        "Turn two cards over. A mismatch stays visible until your next selection.".to_owned(),
    );
    view! {<p role="status">{move||message.get()}</p><div class="memory-grid">{cards.iter().enumerate().map(|(i,card)|{let face=card["face"].as_str().unwrap_or("").to_owned();let label_face=face.clone();let deck=cards.clone();view!{<button class="memory-card" aria-label=move||if state.revealed.get()||open.get().contains(&i)||matched.get().iter().flatten().any(|v|*v==i){format!("Card {}: {}",i+1,label_face)}else{format!("Card {}: face down",i+1)} disabled=move||blocked(state)||matched.get().iter().flatten().any(|v|*v==i) on:click=move|_|{let mut shown=open.get_untracked();if shown.contains(&i){return;}
    if shown.len()==2{shown.clear();}shown.push(i);if shown.len()==2{if deck[shown[0]]["face"]==deck[shown[1]]["face"]{matched.update(|pairs|pairs.push([shown[0],shown[1]]));let pairs=matched.get_untracked().iter().map(|p|json!([deck[p[0]]["id"],deck[p[1]]["id"]])).collect::<Vec<_>>();set(state,json!(pairs));message.set("A match! Those cards stay face up.".into());}else{message.set("Not a match. Remember their positions; your next selection turns them down.".into());}}open.set(shown);}>{move||if state.revealed.get()||open.get().contains(&i)||matched.get().iter().flatten().any(|v|*v==i){face.clone()}else{"?".into()}}</button>}}).collect_view()}</div><button class="text-button" disabled=move||blocked(state) on:click=move|_|{open.set(vec![]);matched.set(vec![]);set(state,json!([]));message.set("Cards reset. Turn two over to begin.".into());}>"Reset cards"</button>}
}
