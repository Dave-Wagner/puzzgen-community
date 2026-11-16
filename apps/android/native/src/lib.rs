mod audio_policy;
mod beacon;
mod garden;
mod garden_controls;
mod garden_walk;
mod garden_weather;
mod home;
mod lantern;
mod lost_lantern;
#[allow(dead_code)]
mod model;
mod my_garden;
mod safe_audio;
mod walk_physics;
mod weather;
mod weather_audio;
mod woodland;
use bevy::audio::AddAudioSource;
use safe_audio::SafeAudio;
mod platform;
mod pointer;
mod view;
use bevy::{
    prelude::*,
    window::{AppLifecycle, PresentMode},
    winit::WinitSettings,
};
use model::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    sync::{Mutex, mpsc},
    time::Duration,
};
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    #[serde(default)]
    lantern_trail: lost_lantern::SavedTrail,
    #[serde(default)]
    sanctuary: puzzgen_progression::Garden,
    #[serde(default)]
    weather: weather::Preferences,
    #[serde(default)]
    garden_control: garden_walk::ControlMode,
    #[serde(default)]
    garden_left_handed: bool,
    game: Option<Game>,
    #[serde(default)]
    boards: Vec<Game>,
    muted: bool,
    reduced_motion: bool,
    large: bool,
    clears: Vec<String>,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            lantern_trail: lost_lantern::SavedTrail::default(),
            sanctuary: puzzgen_progression::Garden::default(),
            weather: weather::Preferences::default(),
            garden_control: garden_walk::ControlMode::default(),
            garden_left_handed: false,
            game: None,
            boards: vec![],
            muted: false,
            reduced_motion: false,
            large: true,
            clears: vec![],
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
enum Page {
    Landmark,
    MyGarden,
    Home,
    Play,
    Help,
    Credits,
    Settings,
    Weather,
}
#[derive(Resource)]
struct Pocket {
    arranging: bool,
    garden_history: Vec<[bool; puzzgen_progression::SPOTS]>,
    saved: Saved,
    page: Page,
    navigation: Vec<Page>,
    word_preview: Option<([usize; 2], [usize; 2])>,
    history: Vec<Game>,
    message: String,
    busy: bool,
    dirty: bool,
    offset: [usize; 2],
    difficulty: usize,
    credits_page: usize,
    home_page: usize,
    ambient: f32,
    celebrate: bool,
    scroll: f32,
    content_height: f32,
}
#[derive(Default)]
struct Gesture {
    action: Option<Action>,
    origin: Option<Vec2>,
    last: Option<Vec2>,
    dragged: bool,
}
#[derive(Component)]
struct Screen;
#[derive(Component)]
struct ScreenContent;
#[derive(Component)]
struct HitRect([f32; 4]);
#[derive(Component, Clone, Debug)]
enum Action {
    LanternVisit(usize),
    LanternRetry,
    LanternCheck,
    LanternPick(String),
    LanternSort(String, String),
    LanternUndo,
    LanternClear,
    MyGarden,
    GardenArrange,
    GardenUndo,
    GardenControl(garden_walk::ControlMode),
    GardenHand,
    Weather,
    WeatherMode(weather::Mood),
    WeatherCity,
    WeatherLocation,
    Thunder,
    Open(usize),
    Home,
    Help,
    Back,
    Credits,
    Settings,
    CreditsNext,
    Tap([usize; 2]),
    Word([usize; 2], [usize; 2]),
    Enter(String),
    Cipher(usize),
    Step(&'static str),
    Pan(isize, isize),
    Mode(bool),
    Check,
    Hint,
    Undo,
    Restart,
    New,
    Level,
    Mute,
    Motion,
    Notices,
}
#[derive(Component)]
struct Particle {
    velocity: Vec2,
    life: f32,
    total: f32,
    spin: f32,
    gravity: f32,
    bounce: bool,
}
#[derive(Component)]
struct Entrance {
    age: f32,
    target: f32,
}
#[derive(Component)]
struct Chrome {
    kind: usize,
    phase: f32,
}
#[derive(Component)]
struct Music {
    gain: f32,
    outgoing: bool,
}
#[derive(Resource)]
struct MusicState {
    kind: usize,
    foreground: bool,
}
#[derive(Resource)]
struct Canvas {
    height: f32,
    scale: f32,
    offset: Vec2,
}
enum Reply {
    Puzzle(Result<Puzzle, String>),
    Check(Result<bool, String>),
    Hint(Result<Value, String>),
}
#[derive(Resource)]
struct Network {
    tx: mpsc::Sender<Reply>,
    rx: Mutex<mpsc::Receiver<Reply>>,
}
#[derive(Resource)]
struct Art {
    star: Handle<Image>,
    magic: Handle<Image>,
    toy: Handle<Image>,
    stone: Handle<Image>,
    tap: Handle<Image>,
    click: Handle<SafeAudio>,
    win: Handle<SafeAudio>,
}

#[bevy_main]
pub fn main() {
    let saved = platform::load().unwrap_or_default();
    audio_policy::set_muted(saved.muted);
    let (tx, rx) = mpsc::channel();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Beacon & Bough".into(),
                        resolution: (480, 1040).into(),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(native_render_plugin()),
        )
        .add_audio_source::<SafeAudio>()
        // Default features are disabled: glTF scene cloning needs these explicit
        // registrations instead of Bevy's optional reflect_auto_register feature.
        .register_type::<Transform>()
        .register_type::<GlobalTransform>()
        .register_type::<bevy::transform::components::TransformTreeChanged>()
        .register_type::<Visibility>()
        .register_type::<InheritedVisibility>()
        .register_type::<ViewVisibility>()
        .register_type::<bevy::camera::visibility::VisibilityClass>()
        .register_type::<bevy::camera::primitives::Aabb>()
        .register_type::<Mesh3d>()
        .register_type::<MeshMaterial3d<StandardMaterial>>()
        .register_type::<Name>()
        .register_type::<ChildOf>()
        .register_type::<Children>()
        .register_type::<bevy::gltf::GltfMeshName>()
        .register_type::<bevy::gltf::GltfMaterialName>()
        .register_type::<bevy::gltf::GltfExtras>()
        .register_type::<bevy::gltf::GltfMeshExtras>()
        .register_type::<bevy::gltf::GltfMaterialExtras>()
        .register_type::<bevy::gltf::GltfSceneExtras>()
        .add_plugins(MaterialPlugin::<garden_weather::Surface>::default())
        .init_asset_loader::<safe_audio::SafeLoader>()
        .insert_resource(ClearColor(Color::srgb(0.05, 0.12, 0.10)))
        .init_resource::<garden_walk::Walk>()
        .init_resource::<garden_weather::Atmosphere>()
        .init_resource::<garden_weather::SceneChecks>()
        .init_resource::<weather::Feed>()
        .init_resource::<lost_lantern::Feed>()
        .insert_resource(WinitSettings::game())
        .insert_resource(MusicState {
            kind: usize::MAX,
            foreground: false,
        })
        .insert_resource(Canvas {
            height: 1040.,
            scale: 1.,
            offset: Vec2::ZERO,
        })
        .insert_resource(Network {
            tx,
            rx: Mutex::new(rx),
        })
        .insert_resource(Pocket {
            arranging: false,
            garden_history: vec![],
            saved,
            page: Page::Home,
            navigation: vec![],
            word_preview: None,
            history: vec![],
            message: String::new(),
            busy: false,
            dirty: true,
            offset: [0, 0],
            difficulty: 0,
            credits_page: 0,
            home_page: 0,
            ambient: 0.,
            celebrate: false,
            scroll: 0.,
            content_height: 1040.,
        })
        .add_systems(Startup, (setup, home::load))
        .add_systems(
            Update,
            (
                resize,
                (garden_walk::update, my_garden::hide_maze, my_garden::update).chain(),
                weather::update,
                garden_weather::coat,
                garden_weather::validate_scenes,
                garden_weather::animate,
                interact,
                (receive, lost_lantern::update).chain(),
                complete_game,
                view::draw,
                animate,
                particles,
                lifecycle,
                sync_audio_policy,
                music,
                weather_audio::update,
                chrome,
                (home::animate, beacon::animate).chain(),
                garden::animate,
                garden_walk::hud,
            )
                .chain(),
        )
        .run();
}
fn native_render_plugin() -> bevy::render::RenderPlugin {
    #[cfg(all(target_os = "android", any(target_arch = "x86", target_arch = "arm")))]
    {
        use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
        bevy::render::RenderPlugin {
            // Old GLES contexts cannot usefully compile pipelines concurrently.
            synchronous_pipeline_compilation: true,
            render_creation: RenderCreation::Automatic(WgpuSettings {
                backends: Some(Backends::GL),
                ..default()
            }),
            ..default()
        }
    }
    #[cfg(not(all(target_os = "android", any(target_arch = "x86", target_arch = "arm"))))]
    {
        bevy::render::RenderPlugin::default()
    }
}
fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((Camera2d, Msaa::Off, IsDefaultUiCamera));
    commands.insert_resource(Art {
        star: assets.load("art/star.png"),
        magic: assets.load("art/magic.png"),
        toy: assets.load("art/toy.png"),
        stone: assets.load("art/stone.png"),
        tap: assets.load("art/tap.png"),
        click: assets.load("audio/click.ogg"),
        win: assets.load("audio/garden-win.ogg"),
    });
    info!("Puzzgen native Bevy renderer started; no WebView, ads, purchases or analytics");
}
fn resize(
    windows: Query<&Window>,
    mut canvas: ResMut<Canvas>,
    mut scale: ResMut<UiScale>,
    mut p: ResMut<Pocket>,
) {
    let Ok(w) = windows.single() else {
        return;
    };
    let [left, top, right, bottom] = platform::safe_area().map(|v| v / w.scale_factor());
    let s = ((w.width() - left - right) / 480.).max(0.1);
    let h = ((w.height() - top - bottom) / s).max(1.);
    let offset = Vec2::new(left, top);
    if (canvas.height - h).abs() > 1. || (canvas.scale - s).abs() > 0.01 || canvas.offset != offset
    {
        canvas.height = h;
        canvas.scale = s;
        canvas.offset = offset;
        scale.0 = s;
        p.dirty = true;
    }
}
fn persist(p: &Pocket) {
    let mut saved = p.saved.clone();
    if let Some(game) = &saved.game {
        saved.boards.retain(|g| g.puzzle.kind != game.puzzle.kind);
        saved.boards.push(game.clone());
    }
    platform::save(&saved);
}
fn remember(p: &mut Pocket) {
    if let Some(g) = p.saved.game.clone() {
        if p.history.len() >= 100 {
            p.history.remove(0);
        }
        p.history.push(g);
    }
}
fn sound(commands: &mut Commands, art: &Art, p: &Pocket, win: bool) {
    if !p.saved.muted && audio_policy::allowed() {
        commands.spawn((
            AudioPlayer::<SafeAudio>(if win {
                art.win.clone()
            } else {
                art.click.clone()
            }),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(if win {
                0.35
            } else {
                0.12
            })),
        ));
    }
}
fn award(p: &mut Pocket) {
    let mut puzzle_reward = None;
    if let Some(g) = &mut p.saved.game {
        if g.won {
            return;
        }
        g.won = true;
        puzzle_reward = Some((
            g.puzzle.kind.clone(),
            g.puzzle.seed.clone(),
            g.puzzle.difficulty.clone(),
        ));
        let id = format!(
            "{}:{}:{}",
            g.puzzle.kind, g.puzzle.seed, g.puzzle.difficulty
        );
        if !p.saved.clears.contains(&id) {
            p.saved.clears.push(id);
            if p.saved.clears.len() > 2000 {
                p.saved.clears.remove(0);
            }
        }
    }
    p.message = if let Some((kind, seed, difficulty)) = puzzle_reward {
        if p.saved.sanctuary.complete_puzzle(&kind, &seed, &difficulty) {
            "The beacon celebrates you. +10 Care points and a lantern for My Garden.".into()
        } else {
            "Lovely to see you again. This puzzle's keepsake is already yours.".into()
        }
    } else {
        "Beautifully done! Enjoy another whenever you like.".into()
    };
    p.celebrate = true;
}
#[allow(clippy::too_many_arguments)]
fn interact(
    mut commands: Commands,
    mut p: ResMut<Pocket>,
    network: Res<Network>,
    art: Res<Art>,
    buttons: Query<(&Action, &HitRect)>,
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    canvas: Res<Canvas>,
    mut gesture: Local<Gesture>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut lantern_feed: ResMut<lost_lantern::Feed>,
) {
    if platform::take_back() || keys.just_pressed(KeyCode::Escape) {
        p.word_preview = None;
        if p.page == Page::Home {
            platform::background();
        } else {
            p.page = p.navigation.pop().unwrap_or(Page::Home);
        }
        p.scroll = 0.;
        p.dirty = true;
        persist(&p);
        return;
    }
    if touches.iter_just_canceled().next().is_some() {
        *gesture = Gesture::default();
        p.word_preview = None;
        p.dirty = true;
        return;
    }
    let (press_pos, pos) = pointer::positions(
        touches.iter_just_pressed().next().map(|t| t.position()),
        touches.iter().next().map(|t| t.position()),
        touches.iter_just_released().next().map(|t| t.position()),
        windows.single().ok().and_then(Window::cursor_position),
    );
    let down = mouse.just_pressed(MouseButton::Left) || touches.any_just_pressed();
    if down {
        gesture.action = press_pos.and_then(|pos| {
            let point = (pos - canvas.offset) / canvas.scale + Vec2::new(0., p.scroll);
            if point.x < 0.
                || point.x >= 480.
                || point.y < p.scroll
                || point.y >= p.scroll + canvas.height
            {
                return None;
            }
            buttons
                .iter()
                .find(|(_, r)| {
                    point.x >= r.0[0]
                        && point.x < r.0[0] + r.0[2]
                        && point.y >= r.0[1]
                        && point.y < r.0[1] + r.0[3]
                })
                .map(|(a, _)| a.clone())
        });
        gesture.origin = press_pos;
        gesture.last = press_pos;
        gesture.dragged = false;
        if platform::trace_input() {
            info!(
                "INPUT down={press_pos:?} current={pos:?} offset={:?} scale={} scroll={} action={:?} touch={} mouse={}",
                canvas.offset,
                canvas.scale,
                p.scroll,
                gesture.action,
                touches.any_just_pressed(),
                mouse.just_pressed(MouseButton::Left)
            );
        }
    }
    // The 3D playfield owns movement/look gestures. Only HUD buttons use this
    // tap handler; a walking thumb must never scroll the page.
    if (garden_walk::active(&p) || p.page == Page::MyGarden) && gesture.action.is_none() {
        return;
    }
    let word_drag = p.page == Page::Play
        && p.saved.game.as_ref().is_some_and(|g| g.index() == 4)
        && matches!(gesture.action, Some(Action::Tap(_)));
    let can_scroll = p.page != Page::Home && !word_drag;
    for event in wheel.read() {
        if can_scroll {
            p.scroll =
                (p.scroll - event.y * 35.).clamp(0., (p.content_height - canvas.height).max(0.));
        }
    }
    if (mouse.pressed(MouseButton::Left)
        || touches.iter().next().is_some()
        || touches.any_just_released())
        && let Some(pos) = pos
    {
        if let Some(origin) = gesture.origin
            && pos.distance(origin) > 16. * canvas.scale
        {
            gesture.dragged = true;
        }
        if can_scroll
            && gesture.dragged
            && let Some(last) = gesture.last
        {
            p.scroll = (p.scroll + (last.y - pos.y) / canvas.scale)
                .clamp(0., (p.content_height - canvas.height).max(0.));
        }
        gesture.last = Some(pos);
        if word_drag
            && gesture.dragged
            && let Some(Action::Tap(start)) = gesture.action
        {
            let point = (pos - canvas.offset) / canvas.scale + Vec2::new(0., p.scroll);
            if let Some(end) = buttons.iter().find_map(|(action, r)| {
                if point.x >= r.0[0]
                    && point.x < r.0[0] + r.0[2]
                    && point.y >= r.0[1]
                    && point.y < r.0[1] + r.0[3]
                    && let Action::Tap(cell) = action
                {
                    return Some(*cell);
                }
                None
            }) {
                let preview = Some((start, end));
                if p.word_preview != preview {
                    p.word_preview = preview;
                    p.dirty = true;
                }
            }
        }
    }
    if !(mouse.just_released(MouseButton::Left) || touches.any_just_released()) {
        return;
    }
    let mut candidate = gesture.action.take();
    if platform::trace_input() {
        info!(
            "INPUT release={pos:?} dragged={} candidate={candidate:?} preview={:?}",
            gesture.dragged, p.word_preview
        );
    }
    let was_word_drag = word_drag && gesture.dragged;
    if was_word_drag {
        candidate = p.word_preview.map(|(start, end)| Action::Word(start, end));
    }
    if p.word_preview.take().is_some() {
        p.dirty = true;
    }
    gesture.origin = None;
    gesture.last = None;
    if gesture.dragged && !was_word_drag {
        return;
    }
    let Some(action) = candidate else {
        return;
    };
    let previous_page = p.page;
    let navigating_back = matches!(action, Action::Back);
    let reset_scroll = matches!(&action, Action::Open(_) | Action::New);
    if p.busy
        && !matches!(
            action,
            Action::Mute | Action::Motion | Action::Home | Action::Back | Action::Credits
        )
    {
        return;
    }
    sound(&mut commands, &art, &p, false);
    let (_, accent, _) = view::palette(p.saved.game.as_ref().map_or(0, Game::index));
    if !p.saved.reduced_motion {
        burst(&mut commands, &art, Vec2::new(0., 50.), accent, 12, false);
    }
    match action {
        Action::LanternVisit(slot) => lost_lantern::visit(&mut p, &mut lantern_feed, slot),
        action @ (Action::LanternRetry
        | Action::LanternCheck
        | Action::LanternPick(_)
        | Action::LanternSort(_, _)
        | Action::LanternUndo
        | Action::LanternClear) => lost_lantern::action(&mut p, &mut lantern_feed, &action),
        Action::MyGarden => {
            p.page = Page::MyGarden;
            p.arranging = false;
            p.message = if p.saved.sanctuary.lanterns == 0 {
                "Enjoy any puzzle to bring your first lantern home."
            } else {
                "Your garden is always glad to see you."
            }
            .into();
        }
        Action::GardenArrange => {
            p.arranging = !p.arranging;
            p.message.clear();
        }
        Action::GardenUndo => {
            if let Some(layout) = p.garden_history.pop() {
                p.saved.sanctuary.restore_layout(layout);
                p.message = "Your last arrangement has been undone.".into();
            }
        }
        Action::GardenControl(mode) => p.saved.garden_control = mode,
        Action::GardenHand => p.saved.garden_left_handed = !p.saved.garden_left_handed,
        Action::Weather => p.page = Page::Weather,
        Action::WeatherMode(mode) => {
            platform::cancel_weather_location();
            p.saved.weather.mode = mode;
        }
        Action::WeatherCity => platform::weather_dialog(),
        Action::WeatherLocation => platform::weather_location(),
        Action::Thunder => p.saved.weather.thunder = !p.saved.weather.thunder,
        Action::Open(i) => {
            if !matches!(i, 0..=2 | 4) {
                return;
            }
            if let Some(old) = p.saved.game.clone() {
                p.saved.boards.retain(|g| g.puzzle.kind != old.puzzle.kind);
                p.saved.boards.push(old);
            }
            let level = ["easy", "medium", "hard"][p.difficulty];
            if let Some(saved) = p
                .saved
                .boards
                .iter()
                .find(|g| g.puzzle.kind == KINDS[i] && g.puzzle.difficulty == level)
                .cloned()
            {
                p.saved.game = Some(saved);
                p.page = Page::Play;
                p.offset = [0, 0];
                p.history.clear();
                p.message.clear();
            } else if let Some(puzzle) = fixtures()
                .into_iter()
                .find(|f| f.kind == KINDS[i] && f.difficulty == level)
            {
                p.saved.game = Some(Game::new(puzzle, false));
                p.page = Page::Play;
                p.history.clear();
                p.offset = [0, 0];
                p.message.clear();
                let key = platform::api_key();
                if !key.is_empty() {
                    let path = format!(
                        "/api/v1/puzzles/{}?difficulty={}&include_solution={}",
                        KINDS[i],
                        level,
                        i == 7
                    );
                    p.busy = true;
                    p.message = "A fresh puzzle is on its way...".into();
                    let tx = network.tx.clone();
                    std::thread::spawn(move || {
                        let _ = tx.send(Reply::Puzzle(request(&path, None, &key).and_then(|v| {
                            serde_json::from_value(v)
                                .map_err(|_| "That puzzle couldn't be read.".into())
                        })));
                    });
                }
            }
        }
        Action::Home => {
            p.page = Page::Home;
            p.message.clear();
        }
        Action::Back => {
            p.page = p.navigation.pop().unwrap_or(Page::Home);
        }
        Action::Help => p.page = Page::Help,
        Action::Credits => {
            p.page = Page::Credits;
            p.credits_page = 0;
        }
        Action::CreditsNext => {
            let manifest: Value =
                serde_json::from_str(include_str!("../../game/public/credits.json"))
                    .unwrap_or_default();
            let count = manifest["packs"].as_array().map_or(0, Vec::len) + 3;
            p.credits_page = (p.credits_page + 1) % count;
        }
        Action::Mute => {
            p.saved.muted = !p.saved.muted;
            audio_policy::set_muted(p.saved.muted);
        }
        Action::Motion => p.saved.reduced_motion = !p.saved.reduced_motion,
        Action::Notices => platform::license_dialog(),
        Action::Pan(dr, dc) => {
            if let Some(g) = &p.saved.game {
                let max = if p.saved.large {
                    7
                } else {
                    g.rows().max(g.cols())
                };
                p.offset = [
                    p.offset[0]
                        .saturating_add_signed(dr)
                        .min(g.rows().saturating_sub(max)),
                    p.offset[1]
                        .saturating_add_signed(dc)
                        .min(g.cols().saturating_sub(max)),
                ];
            }
        }
        Action::Undo => {
            if let Some(mut old) = p.history.pop() {
                if let Some(g) = &p.saved.game {
                    old.assisted |= g.assisted;
                }
                p.saved.game = Some(old);
                p.message.clear();
            }
        }
        Action::Restart => {
            if let Some(g) = p.saved.game.clone() {
                p.saved.game = Some(Game::new(g.puzzle, g.live));
                p.history.clear();
                p.message.clear();
                p.offset = [0, 0];
            }
        }
        Action::Level => {
            p.difficulty = (p.difficulty + 1) % 3;
            p.message = "Difficulty will apply to your next puzzle.".into();
        }
        Action::New => {
            if let Some(g) = &p.saved.game {
                let key = platform::api_key();
                if key.is_empty() {
                    let kind = g.index();
                    let level = ["easy", "medium", "hard"][p.difficulty];
                    if let Some(puzzle) = fixtures()
                        .into_iter()
                        .find(|f| f.kind == KINDS[kind] && f.difficulty == level)
                    {
                        p.saved.game = Some(Game::new(puzzle, false));
                        p.offset = [0, 0];
                        p.history.clear();
                        p.message="Practice board ready. Family settings can connect fresh online puzzles.".into();
                    }
                } else {
                    let path = format!(
                        "/api/v1/puzzles/{}?difficulty={}&include_solution={}",
                        g.puzzle.kind,
                        ["easy", "medium", "hard"][p.difficulty],
                        g.puzzle.kind == "minesweeper"
                    );
                    p.busy = true;
                    p.message = "A fresh puzzle is on its way...".into();
                    let tx = network.tx.clone();
                    std::thread::spawn(move || {
                        let reply = request(&path, None, &key).and_then(|v| {
                            serde_json::from_value(v).map_err(|_| "Unexpected puzzle format".into())
                        });
                        let _ = tx.send(Reply::Puzzle(reply));
                    });
                }
            }
        }
        Action::Check => {
            if let Some(g) = &p.saved.game {
                if g.won || g.lost {
                    return;
                }
                if !g.live {
                    if g.practice_correct() {
                        award(&mut p);
                        sound(&mut commands, &art, &p, true);
                    } else {
                        p.message = "Not quite yet. Take your time, or ask for a hint.".into();
                    }
                } else {
                    let g = g.clone();
                    let tx = network.tx.clone();
                    let key = platform::api_key();
                    p.busy = true;
                    std::thread::spawn(move || {
                        let seed = g.puzzle.seed.parse::<u64>().unwrap_or(0);
                        let body = json!({"kind":g.puzzle.kind,"seed":seed,"difficulty":g.puzzle.difficulty,"answer":g.answer()});
                        let reply = request("/api/v1/verify", Some(body), &key)
                            .map(|v| v["correct"] == true);
                        let _ = tx.send(Reply::Check(reply));
                    });
                }
            }
        }
        Action::Hint => {
            if let Some(g) = &p.saved.game {
                if g.won || g.lost {
                    return;
                }
                if g.puzzle.solution.is_some() {
                    remember(&mut p);
                    if let Some(g) = &mut p.saved.game {
                        g.hint();
                    }
                    p.message = "A little helping hand. You're doing fine.".into();
                } else {
                    let path = format!(
                        "/api/v1/puzzles/{}?seed={}&difficulty={}&include_solution=true",
                        g.puzzle.kind, g.puzzle.seed, g.puzzle.difficulty
                    );
                    let tx = network.tx.clone();
                    let key = platform::api_key();
                    p.busy = true;
                    std::thread::spawn(move || {
                        let _ = tx.send(Reply::Hint(
                            request(&path, None, &key).map(|v| v["solution"].clone()),
                        ));
                    });
                }
            }
        }
        Action::Tap(pos) => {
            let before = p.saved.game.clone();
            if p.saved
                .game
                .as_ref()
                .is_some_and(|g| g.index() > 1 && g.index() != 4)
            {
                remember(&mut p);
            }
            if let Some(g) = &mut p.saved.game {
                g.tap(pos);
            }
            p.message.clear();
            if let Some(old) = before.filter(|g| g.index() == 4) {
                let found = p.saved.game.as_ref().map_or(0, |g| g.found.len());
                if found > old.found.len() {
                    p.history.push(old);
                    if p.history.len() > 100 {
                        p.history.remove(0);
                    }
                    p.message = "A word discovered! Nicely spotted.".into();
                } else if old.selected.is_some() {
                    p.message = "Try another first and last letter from a journal word.".into();
                }
            }
        }
        Action::Word(start, end) => {
            if let Some(old) = p.saved.game.clone().filter(|g| g.index() == 4 && !g.won) {
                if let Some(g) = &mut p.saved.game {
                    g.selected = Some(start);
                    g.tap(end);
                }
                if p.saved
                    .game
                    .as_ref()
                    .is_some_and(|g| g.found.len() > old.found.len())
                {
                    p.history.push(old);
                    if p.history.len() > 100 {
                        p.history.remove(0);
                    }
                    p.message = "A word discovered! Nicely spotted.".into();
                } else {
                    p.message = "Look for a journal word in a straight line.".into();
                }
            }
        }
        Action::Enter(v) => {
            remember(&mut p);
            if let Some(g) = &mut p.saved.game {
                g.enter(&v);
            }
            p.message.clear();
        }
        Action::Cipher(index) => {
            if let Some(g) = &mut p.saved.game {
                g.selected = Some([index, 0]);
            }
        }
        Action::Mode(flag) => {
            if let Some(g) = &mut p.saved.game {
                g.alternate = flag;
            }
        }
        Action::Step(d) => {
            remember(&mut p);
            let old_path = p.saved.game.as_ref().map(|g| g.path.clone());
            if let Some(g) = &mut p.saved.game {
                g.step(d);
                if let Some([r, c]) = g.path.last().copied() {
                    p.offset = [
                        r.saturating_sub(3).min(g.rows().saturating_sub(7)),
                        c.saturating_sub(3).min(g.cols().saturating_sub(7)),
                    ];
                }
            }
            if p.saved
                .game
                .as_ref()
                .is_some_and(|g| g.index() == 2 && Some(&g.path) == old_path.as_ref())
            {
                p.history.pop();
            } else {
                p.message.clear();
            }
        }
        Action::Settings => p.page = Page::Settings,
    }
    if previous_page != p.page || reset_scroll {
        p.scroll = 0.;
    }
    if p.page != previous_page {
        if p.page == Page::Home {
            p.navigation.clear();
        } else if !navigating_back {
            p.navigation.push(previous_page);
        }
    }
    p.dirty = true;
    persist(&p);
}
fn request(path: &str, body: Option<Value>, key: &str) -> Result<Value, String> {
    // Firebase acquisition/refresh runs on this HTTP worker. Family keys stay intact.
    let guest;
    let key = if key == "@guest" {
        guest = platform::guest_token();
        if guest.is_empty() {
            return Err("Online play is resting. Your saved puzzle and practice still work.".into());
        }
        guest.as_str()
    } else {
        key
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| {
            "Online puzzles are unavailable. You can keep playing this board.".to_string()
        })?;
    let url = format!("{}{path}", platform::api_origin());
    let req = if let Some(body) = body {
        client.post(url).json(&body)
    } else {
        client.get(url)
    };
    let response = req
        .bearer_auth(key)
        .send()
        .map_err(|_| "No connection just now. Your puzzle is still here.".to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => {
                "Online play is unavailable just now. Your puzzle and practice still work."
            }
            429 => "Online puzzles need a little break. Your current puzzle is still here.",
            _ => "The online service is resting. Please try again later.",
        }
        .into());
    }
    response
        .json()
        .map_err(|_| "That puzzle couldn't be read. Please try again.".into())
}
fn receive(mut commands: Commands, mut p: ResMut<Pocket>, network: Res<Network>, art: Res<Art>) {
    let reply = network.rx.lock().ok().and_then(|r| r.try_recv().ok());
    let Some(reply) = reply else {
        return;
    };
    p.busy = false;
    match reply {
        Reply::Puzzle(Ok(puzzle)) => {
            p.saved.game = Some(Game::new(puzzle, true));
            p.offset = [0, 0];
            p.history.clear();
            p.message = "A brand new online puzzle, just for you.".into();
        }
        Reply::Check(Ok(true)) => {
            award(&mut p);
            sound(&mut commands, &art, &p, true);
        }
        Reply::Check(Ok(false)) => {
            p.message = "Not quite yet. Keep exploring, or ask for a hint.".into()
        }
        Reply::Hint(Ok(solution)) => {
            remember(&mut p);
            if let Some(g) = &mut p.saved.game {
                g.puzzle.solution = Some(solution);
                g.hint();
            }
            p.message = "A little helping hand. You're doing fine.".into();
        }
        Reply::Puzzle(Err(e)) | Reply::Check(Err(e)) | Reply::Hint(Err(e)) => p.message = e,
    }
    p.dirty = true;
    persist(&p);
}
fn complete_game(mut commands: Commands, mut p: ResMut<Pocket>, art: Res<Art>) {
    if p.page == Page::Play
        && !p.busy
        && p.saved
            .game
            .as_ref()
            .is_some_and(|g| !g.won && g.is_solved())
    {
        award(&mut p);
        sound(&mut commands, &art, &p, true);
        p.dirty = true;
        persist(&p);
    }
}
fn burst(
    commands: &mut Commands,
    art: &Art,
    origin: Vec2,
    color: Color,
    count: usize,
    bounce: bool,
) {
    for j in 0..count {
        let a = j as f32 * 2.39996;
        let speed = 25. + (j % 13) as f32 * 9.;
        let life = 1.8 + (j % 7) as f32 * 0.13;
        commands.spawn((
            Sprite {
                image: art.star.clone(),
                color,
                custom_size: Some(Vec2::splat(5. + (j % 5) as f32 * 2.)),
                ..default()
            },
            Transform::from_xyz(origin.x, origin.y, -0.1),
            Visibility::Hidden,
            Particle {
                velocity: Vec2::new(a.cos() * speed, a.sin() * speed + 50.),
                life,
                total: life,
                spin: a.sin() * 3.,
                gravity: if bounce { 90. } else { -9. },
                bounce,
            },
        ));
    }
}
fn particles(
    mut commands: Commands,
    time: Res<Time>,
    mut p: ResMut<Pocket>,
    art: Res<Art>,
    mut query: Query<(
        Entity,
        &mut Transform,
        &mut Sprite,
        &mut Particle,
        &mut Visibility,
    )>,
    windows: Query<&Window>,
) {
    // Styled pages have glints inside the clipped UI viewport. World-space
    // particles bypass that clipping and would leak into the safe-area reserve.
    if matches!(p.page, Page::Home | Page::MyGarden)
        || p.saved
            .game
            .as_ref()
            .is_some_and(|g| p.page == Page::Play && matches!(g.index(), 0..=2 | 4))
    {
        for (entity, _, _, _, _) in &query {
            commands.entity(entity).despawn();
        }
        return;
    }
    let dt = time.delta_secs().min(0.05);
    let Ok(w) = windows.single() else {
        return;
    };
    let width = w.width();
    let height = w.height();
    let [left, top, right, bottom] = platform::safe_area().map(|v| v / w.scale_factor());
    for (e, mut t, mut sprite, mut particle, mut visibility) in &mut query {
        particle.life -= dt;
        if particle.life <= 0. || p.saved.reduced_motion {
            commands.entity(e).despawn();
            continue;
        }
        particle.velocity.y -= particle.gravity * dt;
        t.translation.x += particle.velocity.x * dt;
        t.translation.y += particle.velocity.y * dt;
        t.rotate_z(particle.spin * dt);
        let radius = sprite.custom_size.unwrap_or(Vec2::splat(24.)).length() * 0.5;
        *visibility = if t.translation.x - radius >= -width * 0.5 + left
            && t.translation.x + radius <= width * 0.5 - right
            && t.translation.y - radius >= -height * 0.5 + bottom
            && t.translation.y + radius <= height * 0.5 - top
        {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if particle.bounce && t.translation.y < -height * 0.35 {
            t.translation.y = -height * 0.35;
            particle.velocity.y = particle.velocity.y.abs() * 0.55;
            particle.velocity.x *= 0.8;
        }
        sprite.color = sprite
            .color
            .with_alpha((particle.life / particle.total).min(0.75));
    }
    p.ambient -= dt;
    if p.ambient <= 0. && !p.saved.reduced_motion && query.iter().count() < 140 {
        p.ambient = 0.07;
        let kind = p
            .saved
            .game
            .as_ref()
            .filter(|_| matches!(p.page, Page::Play | Page::Landmark))
            .map_or(0, Game::index);
        let (_, color, _) = view::palette(kind);
        let elapsed = time.elapsed_secs();
        let x = (elapsed * 1.618).sin() * width * 0.48;
        let y = if kind == 2 || kind == 9 {
            -height * 0.4
        } else {
            height * 0.45
        };
        let life = 6.;
        commands.spawn((
            Sprite {
                image: match kind {
                    1 => art.toy.clone(),
                    5 | 7 => art.stone.clone(),
                    9 => art.magic.clone(),
                    _ => art.star.clone(),
                },
                color: color.with_alpha(0.35),
                custom_size: Some(if kind == 3 {
                    Vec2::new(6., 13.)
                } else {
                    Vec2::splat(if kind == 4 {
                        16.
                    } else if kind == 9 {
                        24.
                    } else {
                        10.
                    })
                }),
                ..default()
            },
            Transform::from_xyz(x, y, -0.2),
            Visibility::Hidden,
            Particle {
                velocity: Vec2::new(
                    (elapsed * 2.).cos() * 15.,
                    if kind == 2 || kind == 5 || kind == 9 {
                        25.
                    } else {
                        -18.
                    },
                ),
                life,
                total: life,
                spin: if kind == 8 { 1.7 } else { 0.3 },
                gravity: 0.,
                bounce: false,
            },
        ));
    }
}
fn chrome(
    time: Res<Time>,
    p: Res<Pocket>,
    mut art: Query<(&Chrome, &mut UiTransform, &mut ImageNode)>,
) {
    for (c, mut transform, mut image) in &mut art {
        let t = time.elapsed_secs() + c.phase;
        if p.saved.reduced_motion {
            transform.scale = Vec2::ONE;
            transform.rotation = Rot2::IDENTITY;
            continue;
        }
        transform.rotation = Rot2::radians(if c.kind == 8 || c.kind == 6 {
            t * 0.35
        } else {
            t.sin() * 0.12
        });
        transform.scale = Vec2::splat(1. + (t * 1.8).sin() * 0.12);
        image.color = image.color.with_alpha(0.25 + (t * 1.2).sin() * 0.10);
    }
}
fn animate(
    time: Res<Time>,
    mut nodes: Query<(&mut Node, &mut Entrance), Without<ScreenContent>>,
    mut screens: Query<&mut Node, With<ScreenContent>>,
    p: Res<Pocket>,
) {
    for mut node in &mut screens {
        node.top = px(-p.scroll);
    }
    for (mut node, mut e) in &mut nodes {
        e.age += time.delta_secs();
        let t = if p.saved.reduced_motion {
            1.
        } else {
            (e.age / 0.5).min(1.)
        };
        let ease = 1. - (1. - t).powi(3);
        node.left = px(e.target + (1. - ease) * 36.);
    }
}
fn lifecycle(
    mut messages: MessageReader<AppLifecycle>,
    sinks: Query<&AudioSink>,
    p: Res<Pocket>,
    mut state: ResMut<MusicState>,
) {
    for event in messages.read() {
        match event {
            AppLifecycle::Suspended => {
                state.foreground = false;
                persist(&p);
                for sink in &sinks {
                    sink.pause();
                }
            }
            AppLifecycle::Running => {
                state.foreground = true;
                if !p.saved.muted && audio_policy::allowed() {
                    for sink in &sinks {
                        sink.play();
                    }
                }
            }
            _ => {}
        }
    }
}
fn music(
    mut commands: Commands,
    time: Res<Time>,
    p: Res<Pocket>,
    assets: Res<AssetServer>,
    mut state: ResMut<MusicState>,
    mut tracks: Query<(Entity, &mut Music, Option<&mut AudioSink>)>,
) {
    let kind = p
        .saved
        .game
        .as_ref()
        .filter(|_| matches!(p.page, Page::Play | Page::Landmark))
        .map_or(10, Game::index);
    if kind != state.kind {
        for (_, mut track, _) in &mut tracks {
            track.outgoing = true;
        }
        state.kind = kind;
        commands.spawn((
            AudioPlayer::<SafeAudio>(assets.load::<SafeAudio>(if kind == 10 {
                "audio/home-carefree.mp3".to_string()
            } else if matches!(kind, 0..=2 | 4) {
                "audio/garden-enchanted-journey.mp3".to_string()
            } else {
                format!("audio/world-{kind}.wav")
            })),
            PlaybackSettings {
                paused: true,
                ..PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::Linear(0.))
            },
            Music {
                gain: 0.,
                outgoing: false,
            },
        ));
    }
    for (entity, mut track, sink) in &mut tracks {
        if let Some(mut sink) = sink {
            let silent = p.saved.muted || !state.foreground || !audio_policy::allowed();
            if silent {
                sink.pause();
                track.gain = 0.;
            } else {
                sink.play();
                let step = time.delta_secs().min(0.05);
                track.gain = if track.outgoing {
                    (track.gain - step * 0.3).max(0.)
                } else {
                    (track.gain + step * 0.15).min(0.15)
                };
            }
            let volume = track.gain;
            sink.set_volume(bevy::audio::Volume::Linear(volume));
            if track.outgoing && volume <= 0. {
                commands.entity(entity).despawn();
            }
        }
    }
}
fn sync_audio_policy(windows: Query<&Window>, mut state: ResMut<MusicState>) {
    #[cfg(not(target_os = "android"))]
    audio_policy::set_active(
        windows.single().is_ok_and(|w| w.focused)
            && std::env::var_os("PUZZGEN_PREVIEW_SILENT").is_none(),
    );
    #[cfg(target_os = "android")]
    let _ = windows;
    state.foreground = audio_policy::allowed();
}
