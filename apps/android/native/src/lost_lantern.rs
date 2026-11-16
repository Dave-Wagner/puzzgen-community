//! The Lost Lantern: optional live-API encounters beside the existing garden walk.
use super::*;
use home::{HomeArt, label, node, pill};
use puzzgen_progression::trail::{STOPS, Trail};
const INK: Color = Color::srgb(0.055, 0.20, 0.14);
const CREAM: Color = Color::srgb(0.99, 0.95, 0.84);
const GOLD: Color = Color::srgb(0.95, 0.74, 0.28);
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct SavedTrail {
    pub trail: Trail,
    challenges: Vec<Option<Challenge>>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Challenge {
    puzzle: Encounter,
    answer: Value,
}
#[derive(Clone, Serialize, Deserialize)]
struct Encounter {
    id: String,
    kind: String,
    seed: String,
    difficulty: String,
    data: Value,
}
#[derive(Clone)]
struct Ticket {
    serial: u64,
    trail: String,
    slot: usize,
    seed: u64,
    puzzle_id: Option<String>,
}
struct Response {
    ticket: Ticket,
    result: Result<Value, String>,
}
#[derive(Resource)]
pub(super) struct Feed {
    tx: mpsc::Sender<Response>,
    rx: Mutex<mpsc::Receiver<Response>>,
    pending: Option<Ticket>,
    serial: u64,
    pub near: Option<usize>,
    pub selected: usize,
    pub status: String,
}
impl Default for Feed {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx: Mutex::new(rx),
            pending: None,
            serial: 0,
            near: None,
            selected: 0,
            status: String::new(),
        }
    }
}
#[derive(Component)]
pub(super) struct Landmark {
    slot: usize,
    base: f32,
}
pub(super) fn prepare(p: &mut Pocket, g: &Game) {
    let key = puzzgen_progression::trail::key(&g.puzzle.seed, &g.puzzle.difficulty, &g.grid);
    if p.saved.lantern_trail.trail.key != key {
        let start = serde_json::from_value(g.puzzle.data["start"].clone()).unwrap_or([1, 1]);
        let end = serde_json::from_value(g.puzzle.data["end"].clone())
            .unwrap_or([g.rows().saturating_sub(2), g.cols().saturating_sub(2)]);
        p.saved.lantern_trail = SavedTrail {
            trail: Trail::new(&g.puzzle.seed, &g.puzzle.difficulty, &g.grid, start, end),
            challenges: vec![None; STOPS.len()],
        };
        p.dirty = true;
        persist(p);
    }
}
pub(super) fn decorate(
    c: &mut Commands,
    root: Entity,
    p: &Pocket,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let foot = meshes.add(Cylinder::new(0.20, 0.38).mesh().resolution(10));
    let orb = meshes.add(Sphere::new(0.13).mesh().ico(2).unwrap());
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.70, 0.64, 0.43),
        perceptual_roughness: 0.9,
        ..default()
    });
    for (slot, stop) in p.saved.lantern_trail.trail.stops.iter().enumerate() {
        let x = (stop.cell[1] as f32 + 0.5) * walk_physics::TILE + 0.32;
        let z = (stop.cell[0] as f32 + 0.5) * walk_physics::TILE;
        c.spawn((
            Mesh3d(foot.clone()),
            MeshMaterial3d(stone.clone()),
            Transform::from_xyz(x, 0.19, z),
            ChildOf(root),
        ));
        let glow = materials.add(StandardMaterial {
            base_color: GOLD,
            emissive: LinearRgba::rgb(1.8, 1.0, 0.2),
            unlit: true,
            ..default()
        });
        c.spawn((
            Mesh3d(orb.clone()),
            MeshMaterial3d(glow),
            Transform::from_xyz(x, 0.65, z),
            Landmark { slot, base: 0.65 },
            garden_weather::Special,
            ChildOf(root),
        ));
    }
}
pub(super) fn hud_blocked(p: Vec2, feed: &Feed) -> bool {
    feed.near.is_some() && p.x >= 18. && p.x <= 462. && p.y >= 184. && p.y <= 292.
}
fn start(p: &mut Pocket, feed: &mut Feed, check: bool) {
    if feed.pending.is_some() {
        feed.status = "One little moment—the garden is listening.".into();
        p.dirty = true;
        return;
    }
    let slot = feed.selected;
    let Some(stop) = p.saved.lantern_trail.trail.stops.get(slot) else {
        return;
    };
    if stop.restored {
        feed.status = "This lantern is already home. Its keepsake is safe in My Garden.".into();
        p.dirty = true;
        return;
    }
    let key = platform::api_key();
    if key.is_empty() {
        feed.status = "Online visits are resting. You can still follow the Beacon home.".into();
        p.dirty = true;
        return;
    }
    let cached = p
        .saved
        .lantern_trail
        .challenges
        .get(slot)
        .and_then(Option::as_ref);
    if check && cached.is_none() {
        return;
    }
    let body = if check {
        let challenge = cached.unwrap();
        let mut b = json!({"kind":challenge.puzzle.kind,"seed":stop.seed,"difficulty":"easy","answer":challenge.answer});
        if let Some(version) = challenge.puzzle.data["content_version"].as_str() {
            b["content_version"] = json!(version);
        }
        Some(b)
    } else {
        None
    };
    feed.serial = feed.serial.wrapping_add(1);
    let ticket = Ticket {
        serial: feed.serial,
        trail: p.saved.lantern_trail.trail.key.clone(),
        slot,
        seed: stop.seed,
        puzzle_id: if check {
            cached.map(|c| c.puzzle.id.clone())
        } else {
            None
        },
    };
    let path = if check {
        "/api/v1/verify".into()
    } else {
        format!(
            "/api/v1/puzzles/{}?seed={}&difficulty=easy",
            STOPS[slot].1, stop.seed
        )
    };
    feed.pending = Some(ticket.clone());
    feed.status = if check {
        "The Beacon is checking your helping hand…"
    } else {
        "A garden friend is finding a puzzle for you…"
    }
    .into();
    let tx = feed.tx.clone();
    std::thread::spawn(move || {
        let result = request(&path, body, &key);
        let _ = tx.send(Response { ticket, result });
    });
    p.dirty = true;
}
pub(super) fn visit(p: &mut Pocket, feed: &mut Feed, slot: usize) {
    if !garden_walk::active(p) || feed.near != Some(slot) {
        return;
    }
    feed.selected = slot;
    feed.status = String::new();
    p.page = Page::Landmark;
    if p.saved
        .lantern_trail
        .challenges
        .get(slot)
        .and_then(Option::as_ref)
        .is_none()
    {
        start(p, feed, false);
    }
}
pub(super) fn action(p: &mut Pocket, feed: &mut Feed, action: &Action) {
    if p.page != Page::Landmark {
        return;
    }
    if matches!(action, Action::LanternRetry) {
        start(p, feed, false);
        return;
    }
    if matches!(action, Action::LanternCheck) {
        start(p, feed, true);
        return;
    }
    if feed.pending.is_some()
        || p.saved
            .lantern_trail
            .trail
            .stops
            .get(feed.selected)
            .is_none_or(|s| s.restored)
    {
        return;
    }
    let Some(challenge) = p
        .saved
        .lantern_trail
        .challenges
        .get_mut(feed.selected)
        .and_then(Option::as_mut)
    else {
        return;
    };
    match action {
        Action::LanternPick(id) => {
            let mut ids = challenge.answer.as_array().cloned().unwrap_or_default();
            if challenge.puzzle.kind == "sound-families" {
                if let Some(i) = ids.iter().position(|v| v.as_str() == Some(id)) {
                    ids.remove(i);
                } else {
                    ids.push(json!(id));
                }
            } else if !ids.iter().any(|v| v.as_str() == Some(id)) {
                ids.push(json!(id));
            }
            challenge.answer = json!(ids);
        }
        Action::LanternSort(item, category) => {
            let mut map = challenge.answer.as_object().cloned().unwrap_or_default();
            map.insert(item.clone(), json!(category));
            challenge.answer = Value::Object(map);
        }
        Action::LanternUndo => {
            if let Some(ids) = challenge.answer.as_array_mut() {
                ids.pop();
            }
        }
        Action::LanternClear => {
            challenge.answer = if challenge.puzzle.kind == "word-sorting" {
                json!({})
            } else {
                json!([])
            }
        }
        _ => return,
    }
    feed.status.clear();
    p.dirty = true;
    persist(p);
}
fn encounter(value: Value, slot: usize, seed: u64) -> Result<Encounter, String> {
    let puzzle: Encounter = serde_json::from_value(value)
        .map_err(|_| "This visit couldn't be read. Try again.".to_owned())?;
    if puzzle.kind != STOPS[slot].1
        || puzzle.seed != seed.to_string()
        || puzzle.difficulty != "easy"
        || puzzle.id.is_empty()
        || puzzle.id.len() > 200
    {
        return Err("That reply belonged to a different garden visit. Please try again.".into());
    }
    let entries = match puzzle.kind.as_str() {
        "word-sorting" => &puzzle.data["items"],
        "sound-families" => &puzzle.data["words"],
        _ => &puzzle.data["tiles"],
    };
    let Some(entries) = entries.as_array() else {
        return Err("This visit has no playable choices. Try again.".into());
    };
    if entries.is_empty()
        || entries.len() > 12
        || entries.iter().any(|e| {
            e["id"]
                .as_str()
                .is_none_or(|s| s.is_empty() || s.len() > 40)
                || e["text"].as_str().is_none_or(|s| s.len() > 100)
        })
    {
        return Err("This visit has unfamiliar choices. Try again.".into());
    }
    if puzzle.kind == "word-sorting"
        && puzzle.data["categories"].as_array().is_none_or(|a| {
            a.len() != 2
                || a.iter()
                    .any(|e| !e["id"].is_string() || !e["label"].is_string())
        })
    {
        return Err("This post bag is missing its labels. Try again.".into());
    }
    Ok(puzzle)
}
pub(super) fn update(
    mut p: ResMut<Pocket>,
    mut feed: ResMut<Feed>,
    walk: Res<garden_walk::Walk>,
    time: Res<Time>,
    mut markers: Query<(&Landmark, &mut Transform, &mut Visibility)>,
) {
    let near = if garden_walk::active(&p) {
        let cell = [
            (walk.position.y / walk_physics::TILE).max(0.) as usize,
            (walk.position.x / walk_physics::TILE).max(0.) as usize,
        ];
        p.saved.lantern_trail.trail.nearby(cell)
    } else {
        None
    };
    if near != feed.near {
        feed.near = near;
        p.dirty = true;
    }
    for (marker, mut transform, mut visibility) in &mut markers {
        transform.translation.y = marker.base
            + if p.saved.reduced_motion {
                0.
            } else {
                (time.elapsed_secs() * 1.6 + marker.slot as f32).sin() * 0.045
            };
        let restored = p
            .saved
            .lantern_trail
            .trail
            .stops
            .get(marker.slot)
            .is_some_and(|s| s.restored);
        transform.scale = Vec3::splat(if restored { 0.72 } else { 1. });
        *visibility = if garden_walk::active(&p) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let replies: Vec<_> = feed
        .rx
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .try_iter()
        .collect();
    for response in replies {
        if feed
            .pending
            .as_ref()
            .is_none_or(|t| t.serial != response.ticket.serial)
        {
            continue;
        }
        feed.pending = None;
        let ticket = response.ticket;
        if !p
            .saved
            .lantern_trail
            .trail
            .accepts(&ticket.trail, ticket.slot, ticket.seed)
        {
            continue;
        }
        let outcome = if let Some(ref id) = ticket.puzzle_id {
            if p.saved
                .lantern_trail
                .challenges
                .get(ticket.slot)
                .and_then(Option::as_ref)
                .is_none_or(|c| c.puzzle.id != *id)
            {
                continue;
            }
            response.result.map(|value| {
                if value["correct"].as_bool() == Some(true) {
                    let challenge = p.saved.lantern_trail.challenges[ticket.slot]
                        .as_ref()
                        .unwrap();
                    let kind = challenge.puzzle.kind.clone();
                    let seed = challenge.puzzle.seed.clone();
                    p.saved.lantern_trail.trail.stops[ticket.slot].restored = true;
                    if p.saved.sanctuary.complete_puzzle(&kind, &seed, "easy") {
                        "A lantern is coming home! +10 Care and one lantern for My Garden."
                            .to_owned()
                    } else {
                        "This keepsake is already safe in My Garden. Thank you for visiting again."
                            .into()
                    }
                } else {
                    "Not quite yet. Take your time, or leave it for another visit.".into()
                }
            })
        } else {
            response
                .result
                .and_then(|v| encounter(v, ticket.slot, ticket.seed))
                .map(|puzzle| {
                    let answer = if puzzle.kind == "word-sorting" {
                        json!({})
                    } else {
                        json!([])
                    };
                    p.saved
                        .lantern_trail
                        .challenges
                        .resize_with(STOPS.len(), || None);
                    p.saved.lantern_trail.challenges[ticket.slot] =
                        Some(Challenge { puzzle, answer });
                    "Your garden friend is ready. There is no hurry.".to_owned()
                })
        };
        if feed.selected == ticket.slot {
            feed.status = outcome.unwrap_or_else(|e| e);
        }
        p.dirty = true;
        persist(&p);
    }
}
fn selected(answer: &Value, id: &str) -> bool {
    answer
        .as_array()
        .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(id)))
}
pub(super) fn hud(c: &mut Commands, root: Entity, p: &Pocket, feed: &Feed, art: &HomeArt) {
    let done = p
        .saved
        .lantern_trail
        .trail
        .stops
        .iter()
        .filter(|s| s.restored)
        .count();
    label(
        c,
        root,
        format!("Lantern trail · {done}/3 friends helped"),
        [22., 177., 436., 28.],
        18.,
        CREAM,
        &art.body,
        true,
    );
    if let Some(slot) = feed.near {
        c.spawn((
            node([18., 210., 444., 82.]),
            BackgroundColor(INK.with_alpha(0.88)),
            BorderRadius::all(px(18.)),
            ChildOf(root),
            bevy::ui::FocusPolicy::Pass,
        ));
        let restored = p
            .saved
            .lantern_trail
            .trail
            .stops
            .get(slot)
            .is_some_and(|s| s.restored);
        pill(
            c,
            root,
            &format!(
                "{}{}",
                if restored { "Visit " } else { "Help at " },
                STOPS[slot].0
            ),
            [28., 220., 424., 58.],
            Action::LanternVisit(slot),
            if restored { GOLD } else { CREAM },
            INK,
            &art.body,
        );
    }
}
pub(super) fn draw(
    c: &mut Commands,
    root: Entity,
    p: &mut Pocket,
    canvas: &Canvas,
    art: &HomeArt,
    feed: &Feed,
) {
    p.content_height = canvas.height.max(1300.);
    c.spawn((
        node([0., 0., 480., p.content_height]),
        BackgroundColor(CREAM),
        ChildOf(root),
        bevy::ui::FocusPolicy::Pass,
    ));
    beacon::draw(c, root, p, art, [24., 22., 48., 48.]);
    label(
        c,
        root,
        "The Lost Lantern",
        [82., 24., 370., 48.],
        32.,
        INK,
        &art.display,
        false,
    );
    pill(
        c,
        root,
        "Another time",
        [24., 88., 210., 54.],
        Action::Back,
        INK,
        CREAM,
        &art.body,
    );
    pill(
        c,
        root,
        "My Garden",
        [252., 88., 204., 54.],
        Action::MyGarden,
        INK,
        CREAM,
        &art.body,
    );
    let slot = feed.selected.min(2);
    label(
        c,
        root,
        STOPS[slot].0,
        [24., 164., 432., 45.],
        31.,
        INK,
        &art.display,
        true,
    );
    let story = match slot {
        0 => {
            "The wind chimes have forgotten which notes belong together. Help them find their rhyming friends."
        }
        1 => {
            "Our garden post needs a kind helper. Put each little delivery into its matching post bag."
        }
        _ => {
            "The story leaves have tumbled in the breeze. Arrange their words into a gentle sentence."
        }
    };
    label(
        c,
        root,
        story,
        [28., 220., 424., 112.],
        23.,
        INK,
        &art.body,
        false,
    );
    let restored = p
        .saved
        .lantern_trail
        .trail
        .stops
        .get(slot)
        .is_some_and(|s| s.restored);
    let pending = feed.pending.is_some();
    if restored {
        label(
            c,
            root,
            "A little light, brought home",
            [24., 350., 432., 60.],
            30.,
            INK,
            &art.display,
            true,
        );
        label(
            c,
            root,
            "Your helping hand earned a lantern for your personal garden. This friend remembers your kindness; replaying this visit never takes or adds another keepsake.",
            [28., 425., 424., 165.],
            24.,
            INK,
            &art.body,
            false,
        );
        pill(
            c,
            root,
            "Place my lantern",
            [52., 620., 376., 62.],
            Action::MyGarden,
            INK,
            CREAM,
            &art.display,
        );
    } else if let Some(challenge) = p
        .saved
        .lantern_trail
        .challenges
        .get(slot)
        .and_then(Option::as_ref)
    {
        let data = &challenge.puzzle.data;
        let answer = &challenge.answer;
        let mut y = 350.;
        if slot == 0 {
            label(
                c,
                root,
                format!(
                    "What rhymes with “{}”?",
                    data["anchor"].as_str().unwrap_or("")
                ),
                [24., y, 432., 55.],
                27.,
                INK,
                &art.display,
                true,
            );
            y += 70.;
            label(
                c,
                root,
                "Choose three words. Tap again to unchoose. (US English)",
                [28., y, 424., 65.],
                20.,
                INK,
                &art.body,
                false,
            );
            y += 80.;
            for (i, word) in data["words"].as_array().into_iter().flatten().enumerate() {
                let id = word["id"].as_str().unwrap_or("");
                let text = word["text"].as_str().unwrap_or("");
                let r = [
                    24. + (i % 2) as f32 * 224.,
                    y + (i / 2) as f32 * 76.,
                    208.,
                    62.,
                ];
                if pending {
                    label(c, root, text, r, 23., INK, &art.body, true);
                } else {
                    pill(
                        c,
                        root,
                        &format!("{}{}", if selected(answer, id) { "✓ " } else { "" }, text),
                        r,
                        Action::LanternPick(id.into()),
                        if selected(answer, id) {
                            GOLD
                        } else {
                            Color::WHITE
                        },
                        INK,
                        &art.body,
                    );
                }
            }
            y += ((data["words"].as_array().map_or(0, Vec::len) + 1) / 2) as f32 * 76.;
        } else if slot == 1 {
            let categories = data["categories"].as_array().unwrap();
            for item in data["items"].as_array().into_iter().flatten() {
                let id = item["id"].as_str().unwrap_or("");
                label(
                    c,
                    root,
                    item["text"].as_str().unwrap_or(""),
                    [26., y, 428., 33.],
                    25.,
                    INK,
                    &art.display,
                    false,
                );
                y += 38.;
                for (i, category) in categories.iter().enumerate() {
                    let cat = category["id"].as_str().unwrap_or("");
                    let chosen = answer[id].as_str() == Some(cat);
                    let r = [24. + i as f32 * 224., y, 208., 55.];
                    let text = category["label"].as_str().unwrap_or("");
                    if pending {
                        label(c, root, text, r, 18., INK, &art.body, true);
                    } else {
                        pill(
                            c,
                            root,
                            &format!("{}{}", if chosen { "✓ " } else { "" }, text),
                            r,
                            Action::LanternSort(id.into(), cat.into()),
                            if chosen { GOLD } else { Color::WHITE },
                            INK,
                            &art.body,
                        );
                    }
                }
                y += 74.;
            }
        } else {
            let tiles = data["tiles"].as_array().unwrap();
            let text = answer
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|id| {
                    tiles
                        .iter()
                        .find(|t| t["id"] == *id)
                        .and_then(|t| t["text"].as_str())
                })
                .collect::<Vec<_>>()
                .join(" ");
            label(
                c,
                root,
                if text.is_empty() {
                    "Your sentence will appear here."
                } else {
                    &text
                },
                [28., y, 424., 96.],
                27.,
                INK,
                &art.body,
                true,
            );
            y += 115.;
            for (i, tile) in tiles.iter().enumerate() {
                let id = tile["id"].as_str().unwrap_or("");
                let r = [
                    24. + (i % 2) as f32 * 224.,
                    y + (i / 2) as f32 * 76.,
                    208.,
                    62.,
                ];
                let text = tile["text"].as_str().unwrap_or("");
                if pending || selected(answer, id) {
                    label(c, root, text, r, 23., INK.with_alpha(0.6), &art.body, true);
                } else {
                    pill(
                        c,
                        root,
                        text,
                        r,
                        Action::LanternPick(id.into()),
                        Color::WHITE,
                        INK,
                        &art.body,
                    );
                }
            }
            y += 3. * 76.;
            if !pending {
                pill(
                    c,
                    root,
                    "Undo word",
                    [24., y, 208., 54.],
                    Action::LanternUndo,
                    Color::WHITE,
                    INK,
                    &art.body,
                );
                y += 72.;
            }
        }
        if !pending {
            pill(
                c,
                root,
                "Help this friend",
                [24., y, 282., 60.],
                Action::LanternCheck,
                INK,
                CREAM,
                &art.display,
            );
            pill(
                c,
                root,
                "Clear",
                [324., y, 132., 60.],
                Action::LanternClear,
                Color::WHITE,
                INK,
                &art.body,
            );
        }
        y += 85.;
        label(
            c,
            root,
            &feed.status,
            [28., y, 424., 120.],
            22.,
            INK,
            &art.body,
            false,
        );
        p.content_height = canvas.height.max(y + 150.);
    } else {
        label(
            c,
            root,
            &feed.status,
            [28., 360., 424., 180.],
            24.,
            INK,
            &art.body,
            false,
        );
        if !pending {
            pill(
                c,
                root,
                "Try this visit again",
                [35., 560., 410., 62.],
                Action::LanternRetry,
                INK,
                CREAM,
                &art.body,
            );
        }
    }
}
