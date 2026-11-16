use super::*;
use bevy::color::Luminance;
pub(super) fn palette(i: usize) -> (Color, Color, Color) {
    let (bg, a) = match i {
        0 => ([0.06, 0.16, 0.11], [0.67, 0.85, 0.44]),
        1 => ([0.17, 0.10, 0.23], [1., 0.73, 0.50]),
        2 => ([0.035, 0.08, 0.10], [1., 0.72, 0.30]),
        3 => ([0.17, 0.07, 0.22], [0.91, 0.66, 0.98]),
        4 => ([0.04, 0.14, 0.09], [0.72, 0.89, 0.42]),
        5 => ([0.025, 0.13, 0.16], [0.39, 0.87, 0.82]),
        6 => ([0.015, 0.04, 0.12], [0.27, 0.88, 1.]),
        7 => ([0.14, 0.11, 0.05], [1., 0.77, 0.30]),
        8 => ([0.18, 0.075, 0.035], [1., 0.65, 0.31]),
        _ => ([0.10, 0.035, 0.19], [0.82, 0.65, 1.]),
    };
    (
        Color::srgb(bg[0], bg[1], bg[2]),
        Color::srgb(a[0], a[1], a[2]),
        Color::srgb(0.97, 0.97, 0.90),
    )
}
#[allow(clippy::too_many_arguments)]
fn text(
    c: &mut Commands,
    parent: Entity,
    value: impl Into<String>,
    x: f32,
    y: f32,
    w: f32,
    size: f32,
    color: Color,
) {
    c.spawn((
        Text::new(value),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
        Node {
            position_type: PositionType::Absolute,
            left: px(x),
            top: px(y),
            width: px(w),
            ..default()
        },
        ChildOf(parent),
    ));
}
#[allow(clippy::too_many_arguments)]
fn button(
    c: &mut Commands,
    parent: Entity,
    label: impl Into<String>,
    r: [f32; 4],
    action: Action,
    bg: Color,
    fg: Color,
    size: f32,
) {
    c.spawn((
        Button,
        action,
        HitRect(r),
        Node {
            position_type: PositionType::Absolute,
            left: px(r[0]),
            top: px(r[1]),
            width: px(r[2]),
            height: px(r[3]),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::all(px(3.)),
            ..default()
        },
        BackgroundColor(bg),
        BorderRadius::all(px(10.)),
        ChildOf(parent),
    ))
    .with_children(|c| {
        c.spawn((
            Text::new(label),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(fg),
            TextLayout::new_with_justify(Justify::Center),
        ));
    });
}
#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    mut commands: Commands,
    mut p: ResMut<Pocket>,
    canvas: Res<Canvas>,
    screen: Query<Entity, With<Screen>>,
    mut clear: ResMut<ClearColor>,
    art: Res<Art>,
    home_art: Res<home::HomeArt>,
    weather_feed: Res<weather::Feed>,
    lantern_feed: Res<lost_lantern::Feed>,
) {
    if !p.dirty {
        return;
    }
    p.dirty = false;
    for e in &screen {
        commands.entity(e).despawn();
    }
    let i = p.saved.game.as_ref().map_or(0, Game::index);
    let (bg, accent, fg) = palette(if p.page == Page::Play { i } else { 0 });
    clear.0 = bg;
    let panel = bg.lighter(0.065);
    let dim = Color::srgb(0.72, 0.80, 0.73);
    let h = canvas.height;
    p.content_height = match p.page {
        Page::Landmark => h.max(1300.),
        Page::MyGarden => h,
        Page::Home => h,
        Page::Help => h.max(1000.),
        Page::Credits => h.max(1150.),
        Page::Settings => h.max(1050.),
        Page::Weather => h.max(1110.),
        Page::Play => 1500.,
    };
    let viewport = commands
        .spawn((
            Screen,
            Node {
                width: px(480.),
                left: px(canvas.offset.x / canvas.scale),
                // The viewport stays inside the safe area. Only its content
                // scrolls; moving both makes drawing and hit testing disagree.
                top: px(canvas.offset.y / canvas.scale),
                height: px(h),
                overflow: Overflow::clip(),
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .id();
    let root = commands
        .spawn((
            ScreenContent,
            Node {
                width: px(480.),
                height: px(h.max(1500.)),
                position_type: PositionType::Absolute,
                top: px(-p.scroll),
                ..default()
            },
            ChildOf(viewport),
        ))
        .id();
    if p.page == Page::Landmark {
        lost_lantern::draw(
            &mut commands,
            root,
            &mut p,
            &canvas,
            &home_art,
            &lantern_feed,
        );
        return;
    }
    if p.page == Page::Home {
        clear.0 = Color::srgb(0.05, 0.12, 0.10);
        home::draw(&mut commands, root, &mut p, &canvas, &home_art);
        return;
    }
    if p.page == Page::Play && p.saved.game.as_ref().is_some_and(|g| g.index() == 2) {
        p.celebrate = false;
        lantern::draw(
            &mut commands,
            root,
            &mut p,
            &canvas,
            &home_art,
            &lantern_feed,
        );
        return;
    }
    if p.page == Page::MyGarden {
        my_garden::draw(&mut commands, root, &mut p, &canvas, &home_art);
        return;
    }
    if p.page == Page::Play && p.saved.game.as_ref().is_some_and(|g| g.index() == 4) {
        p.celebrate = false;
        woodland::draw(&mut commands, root, &mut p, &canvas, &home_art);
        return;
    }
    if p.page == Page::Play && p.saved.game.as_ref().is_some_and(|g| g.index() <= 1) {
        p.celebrate = false;
        garden::draw(&mut commands, root, &mut p, &canvas, &home_art);
        return;
    }
    text(
        &mut commands,
        root,
        "POCKET WORLDS",
        24.,
        22.,
        335.,
        29.,
        fg,
    );
    button(
        &mut commands,
        root,
        if p.saved.muted {
            "Sound off"
        } else {
            "Sound on"
        },
        [355., 18., 104., 48.],
        Action::Mute,
        panel,
        fg,
        17.,
    );
    if p.page != Page::Home {
        button(
            &mut commands,
            root,
            if p.page == Page::Weather {
                "< Back"
            } else {
                "< Games"
            },
            [20., 76., 135., 52.],
            if p.page == Page::Weather {
                Action::Back
            } else {
                Action::Home
            },
            panel,
            fg,
            21.,
        );
    }
    beacon::draw(&mut commands, root, &p, &home_art, [403., 76., 56., 49.]);
    match p.page {
        Page::MyGarden | Page::Landmark => {}
        Page::Weather => weather::draw(
            &mut commands,
            root,
            &mut p,
            &canvas,
            &home_art,
            &weather_feed,
        ),
        Page::Home => {}
        Page::Play => {
            if let Some(g) = p.saved.game.clone() {
                commands.spawn((
                    ImageNode {
                        image: if i == 9 {
                            art.magic.clone()
                        } else if i == 1 {
                            art.toy.clone()
                        } else {
                            art.star.clone()
                        },
                        color: accent.with_alpha(0.3),
                        ..default()
                    },
                    UiTransform::IDENTITY,
                    Chrome {
                        kind: i,
                        phase: i as f32,
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(384.),
                        top: px(139.),
                        width: px(64.),
                        height: px(64.),
                        ..default()
                    },
                    bevy::ui::FocusPolicy::Pass,
                    ChildOf(root),
                ));
                button(
                    &mut commands,
                    root,
                    "How to play",
                    [302., 76., 158., 52.],
                    Action::Help,
                    panel,
                    fg,
                    20.,
                );
                text(&mut commands, root, TITLES[i], 23., 147., 440., 30., accent);
                text(
                    &mut commands,
                    root,
                    format!(
                        "{} | {}",
                        NAMES[i],
                        if g.live {
                            "Online puzzle"
                        } else {
                            "Practice puzzle"
                        }
                    ),
                    24.,
                    191.,
                    435.,
                    18.,
                    dim,
                );
                let bottom = board(&mut commands, root, &g, &p, &art, 233., accent, fg, bg);
                let end = controls(&mut commands, root, &g, bottom + 14., accent, fg, bg);
                let y = end + 12.;
                p.content_height = (y + 390.).max(h);
                if g.won {
                    text(
                        &mut commands,
                        root,
                        "Beautifully done!",
                        24.,
                        y,
                        435.,
                        32.,
                        accent,
                    );
                } else if g.lost {
                    text(
                        &mut commands,
                        root,
                        "Let's try that again.",
                        24.,
                        y,
                        435.,
                        27.,
                        accent,
                    );
                } else {
                    button(
                        &mut commands,
                        root,
                        if p.busy {
                            "Please wait..."
                        } else {
                            "Check my puzzle"
                        },
                        [22., y, 436., 57.],
                        Action::Check,
                        accent,
                        bg,
                        24.,
                    );
                }
                for (j, (label, a)) in [
                    ("Undo", Action::Undo),
                    ("Hint", Action::Hint),
                    ("Start again", Action::Restart),
                ]
                .into_iter()
                .enumerate()
                {
                    button(
                        &mut commands,
                        root,
                        label,
                        [22. + j as f32 * 148., y + 71., 140., 51.],
                        a,
                        panel,
                        fg,
                        20.,
                    );
                }
                button(
                    &mut commands,
                    root,
                    "New puzzle",
                    [22., y + 137., 278., 57.],
                    Action::New,
                    panel,
                    fg,
                    23.,
                );
                button(
                    &mut commands,
                    root,
                    ["Easy", "Medium", "Hard"][p.difficulty],
                    [314., y + 137., 144., 57.],
                    Action::Level,
                    panel,
                    fg,
                    21.,
                );
                text(
                    &mut commands,
                    root,
                    if p.message.is_empty() {
                        "Take your time. There is no clock or penalty."
                    } else {
                        &p.message
                    },
                    24.,
                    y + 211.,
                    432.,
                    20.,
                    fg,
                );
                text(
                    &mut commands,
                    root,
                    "Swipe up or down to see all controls.",
                    24.,
                    y + 300.,
                    432.,
                    17.,
                    dim,
                );
            }
        }
        Page::Help => {
            if let Some(g) = &p.saved.game {
                text(
                    &mut commands,
                    root,
                    format!("How to play {}", NAMES[g.index()]),
                    24.,
                    160.,
                    430.,
                    31.,
                    accent,
                );
                text(
                    &mut commands,
                    root,
                    HELP[g.index()],
                    24.,
                    225.,
                    430.,
                    27.,
                    fg,
                );
                text(
                    &mut commands,
                    root,
                    "You can ask for a hint at any time. No timer, no losing points, and no rush.",
                    24.,
                    555.,
                    430.,
                    26.,
                    dim,
                );
                commands.spawn((
                    ImageNode::new(art.tap.clone()),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(210.),
                        top: px(710.),
                        width: px(60.),
                        height: px(60.),
                        ..default()
                    },
                    ChildOf(root),
                ));
                button(
                    &mut commands,
                    root,
                    "Back to my puzzle",
                    [24., h.max(950.) - 100., 432., 64.],
                    Action::Back,
                    accent,
                    bg,
                    26.,
                );
            }
        }
        Page::Credits => {
            text(
                &mut commands,
                root,
                "Made with care",
                24.,
                160.,
                430.,
                34.,
                accent,
            );
            let manifest: Value =
                serde_json::from_str(include_str!("../../game/public/credits.json"))
                    .unwrap_or_default();
            let packs = manifest["packs"].as_array().cloned().unwrap_or_default();
            if let Some(pack) = packs.get(p.credits_page) {
                text(
                    &mut commands,
                    root,
                    pack["name"].as_str().unwrap_or(""),
                    24.,
                    230.,
                    430.,
                    30.,
                    fg,
                );
                text(
                    &mut commands,
                    root,
                    format!(
                        "{}\n\n{}\n\n{}\n\n{}",
                        pack["creator"]
                            .as_str()
                            .unwrap_or("Kenney Vleugels / Kenney.nl"),
                        pack["license"]
                            .as_str()
                            .unwrap_or("Creative Commons Zero (CC0 1.0)"),
                        pack["usage"].as_str().unwrap_or(""),
                        pack["source"].as_str().unwrap_or("")
                    ),
                    24.,
                    295.,
                    430.,
                    23.,
                    dim,
                );
            } else if p.credits_page == packs.len() {
                text(
                    &mut commands,
                    root,
                    "Home music · Carefree",
                    24.,
                    230.,
                    430.,
                    30.,
                    fg,
                );
                text(
                    &mut commands,
                    root,
                    "Kevin MacLeod (incompetech.com)\n\nLicensed under Creative Commons: By Attribution 4.0\nhttps://creativecommons.org/licenses/by/4.0/\n\nISRC USUAN1400037\n\nHome-screen music candidate, pending review. Recording unchanged; playback fades and volume adjustment are applied. No composer endorsement is implied.",
                    24.,
                    295.,
                    430.,
                    23.,
                    dim,
                );
            } else if p.credits_page == packs.len() + 1 {
                text(
                    &mut commands,
                    root,
                    "Enchanted Journey",
                    24.,
                    230.,
                    430.,
                    30.,
                    fg,
                );
                text(
                    &mut commands,
                    root,
                    "Toybox Garden music\nKevin MacLeod (incompetech.com)\n\nLicensed under Creative Commons: By Attribution 4.0\nhttps://creativecommons.org/licenses/by/4.0/\n\nISRC USUAN1100799\n\nMusic candidate, pending review. Recording unchanged; playback fades and volume adjustment are applied. No composer endorsement is implied.",
                    24.,
                    295.,
                    430.,
                    23.,
                    dim,
                );
            } else {
                text(
                    &mut commands,
                    root,
                    "Created by Dave Wagner for family, curious players and people learning to build games.\n\nRust + Bevy provide native GPU rendering and audio.\n\nNo advertising, purchases or tracking SDKs. Puzzles use the personally hosted Puzzgen API. Optional saved-city weather uses Open-Meteo and GeoNames.\n\nOriginal licenses and file hashes are bundled in the APK. Bevy and its bundled font are open source; notices are included.",
                    24.,
                    230.,
                    430.,
                    23.,
                    fg,
                );
            }
            button(
                &mut commands,
                root,
                "Next credits page >",
                [24., h.max(1000.) - 100., 432., 64.],
                Action::CreditsNext,
                panel,
                fg,
                25.,
            );
            button(
                &mut commands,
                root,
                "Read original licenses",
                [24., h.max(1000.) - 175., 432., 60.],
                Action::Notices,
                panel,
                fg,
                23.,
            );
        }
        Page::Settings => {
            text(
                &mut commands,
                root,
                "Settings",
                24.,
                158.,
                430.,
                32.,
                accent,
            );
            text(
                &mut commands,
                root,
                "These choices stay on this device.",
                24.,
                208.,
                430.,
                22.,
                dim,
            );
            for (j, (label, a)) in [
                (
                    if p.saved.muted {
                        "Sound: OFF"
                    } else {
                        "Sound: ON"
                    },
                    Action::Mute,
                ),
                (
                    if p.saved.reduced_motion {
                        "Gentle motion: ON"
                    } else {
                        "Full animation: ON"
                    },
                    Action::Motion,
                ),
                ("Garden weather", Action::Weather),
                ("Credits and licenses", Action::Credits),
            ]
            .into_iter()
            .enumerate()
            {
                button(
                    &mut commands,
                    root,
                    label,
                    [24., 270. + j as f32 * 83., 432., 66.],
                    a,
                    panel,
                    fg,
                    25.,
                );
            }
            text(
                &mut commands,
                root,
                "Play at your own pace. Your puzzles and preferences are saved on this device.",
                24.,
                705.,
                430.,
                21.,
                dim,
            );
            if !p.message.is_empty() {
                text(
                    &mut commands,
                    root,
                    &p.message,
                    24.,
                    900.,
                    430.,
                    20.,
                    accent,
                );
            }
        }
    }
    if p.celebrate {
        p.celebrate = false;
        if !p.saved.reduced_motion {
            burst(&mut commands, &art, Vec2::new(0., 100.), accent, 100, true);
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn board(
    c: &mut Commands,
    parent: Entity,
    g: &Game,
    p: &Pocket,
    art: &Art,
    top: f32,
    a: Color,
    fg: Color,
    bg: Color,
) -> f32 {
    if g.puzzle.kind == "arithmetic" {
        text(
            c,
            parent,
            g.puzzle.data["expression"].as_str().unwrap_or(""),
            26.,
            top + 30.,
            430.,
            44.,
            fg,
        );
        text(
            c,
            parent,
            if g.text.is_empty() {
                "Your answer: ?".into()
            } else {
                format!("Your answer: {}", g.text)
            },
            26.,
            top + 110.,
            430.,
            36.,
            a,
        );
        return top + 180.;
    }
    if g.puzzle.kind == "cryptogram" {
        let cipher = g.puzzle.data["ciphertext"].as_str().unwrap_or("");
        let letters = g.cipher_letters();
        let mut x = 22.;
        let mut y = top;
        for word in cipher.split(' ') {
            if x + word.len() as f32 * 41. > 460. {
                x = 22.;
                y += 82.;
            }
            for ch in word.chars() {
                let index = letters.iter().position(|l| *l == ch);
                let plain = g
                    .mapping
                    .iter()
                    .find(|(k, _)| *k == ch)
                    .map_or('_', |(_, v)| *v);
                if let Some(index) = index {
                    let selected = g.selected == Some([index, 0]);
                    button(
                        c,
                        parent,
                        format!("{plain}\n{ch}"),
                        [x, y, 38., 70.],
                        Action::Cipher(index),
                        if selected { a } else { bg.lighter(0.13) },
                        if selected { bg } else { fg },
                        23.,
                    );
                }
                x += 41.;
            }
            x += 13.;
        }
        return y + 85.;
    }
    let max = if p.saved.large {
        7
    } else {
        g.cols().max(g.rows())
    };
    let cols = g.cols().min(max);
    let rows = g.rows().min(max);
    let nonogram = g.puzzle.kind == "nonogram";
    let clue = if nonogram { 65. } else { 0. };
    let cell = (436. - clue) / cols as f32;
    let y = top + if nonogram { 64. } else { 0. };
    for rr in 0..rows {
        let r = rr + p.offset[0];
        if r >= g.rows() {
            continue;
        }
        if nonogram {
            let line = clues(&g.puzzle.data["row_clues"][r], " ");
            text(
                c,
                parent,
                line,
                21.,
                y + rr as f32 * cell + cell * 0.3,
                63.,
                19.,
                a,
            );
        }
        for cc in 0..cols {
            let col = cc + p.offset[1];
            if col >= g.cols() {
                continue;
            }
            if nonogram && rr == 0 {
                text(
                    c,
                    parent,
                    clues(&g.puzzle.data["column_clues"][col], "\n"),
                    22. + clue + cc as f32 * cell + cell * 0.28,
                    top,
                    cell,
                    18.,
                    a,
                );
            }
            let (label, color, ink) = cell_style(g, [r, col], a, fg, bg);
            let rect = [
                22. + clue + cc as f32 * cell,
                y + rr as f32 * cell,
                cell - 2.,
                cell - 2.,
            ];
            button(
                c,
                parent,
                label,
                rect,
                Action::Tap([r, col]),
                color,
                ink,
                (cell * 0.45).clamp(15., 39.),
            );
            if g.puzzle.kind == "sliding-puzzle" && g.grid[r][col] != 0 {
                c.spawn((
                    ImageNode {
                        image: art.stone.clone(),
                        color: Color::srgba(1., 1., 1., 0.08),
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(rect[0] + 3.),
                        top: px(rect[1] + 3.),
                        width: px(rect[2] - 6.),
                        height: px(rect[3] - 6.),
                        ..default()
                    },
                    bevy::ui::FocusPolicy::Pass,
                    ChildOf(parent),
                ));
            }
        }
    }
    let mut bottom = y + rows as f32 * cell;
    if g.rows() > rows || g.cols() > cols {
        for (j, (label, dr, dc)) in [("<", 0, -3), ("Up", -3, 0), ("Down", 3, 0), (">", 0, 3)]
            .into_iter()
            .enumerate()
        {
            button(
                c,
                parent,
                label,
                [22. + j as f32 * 110., bottom + 7., 104., 48.],
                Action::Pan(dr, dc),
                bg.lighter(0.08),
                fg,
                21.,
            );
        }
        text(
            c,
            parent,
            format!(
                "Rows {}-{} / {}  Columns {}-{} / {}",
                p.offset[0] + 1,
                (p.offset[0] + rows).min(g.rows()),
                g.rows(),
                p.offset[1] + 1,
                (p.offset[1] + cols).min(g.cols()),
                g.cols()
            ),
            24.,
            bottom + 61.,
            430.,
            16.,
            fg,
        );
        bottom += 92.;
    }
    bottom
}
fn clues(v: &Value, sep: &str) -> String {
    v.as_array()
        .map(|a| a.iter().map(Value::to_string).collect::<Vec<_>>().join(sep))
        .unwrap_or_default()
}
fn cell_style(g: &Game, p: [usize; 2], a: Color, fg: Color, bg: Color) -> (String, Color, Color) {
    let [r, c] = p;
    let value = g.grid[r][c];
    let mut color = bg.lighter(0.09);
    let mut ink = fg;
    let mut label = String::new();
    match g.puzzle.kind.as_str() {
        "sudoku" | "mini-sudoku" => {
            if value != 0 {
                label = value.to_string();
            }
            if g.puzzle.data["grid"][r][c] != 0 {
                color = bg.lighter(0.18);
            } else {
                ink = a;
            }
            let size = g.puzzle.data["box_size"].as_u64().unwrap_or(3) as usize;
            if (r / size + c / size).is_multiple_of(2) {
                color = color.lighter(0.04);
            }
        }
        "maze" => {
            color = if value == 1 {
                bg.lighter(0.18)
            } else {
                bg.darker(0.015)
            };
            if g.path.contains(&p) {
                color = a.darker(0.65);
            }
            if g.path.last() == Some(&p) {
                label = "O".into();
                ink = a;
            } else if json!(p) == g.puzzle.data["end"] {
                label = "GO".into();
                color = a;
                ink = bg;
            }
        }
        "nonogram" => {
            if value == 1 {
                color = a;
            } else if value == -1 {
                label = "x".into();
                ink = a;
            }
        }
        "word-search" => {
            label = g.puzzle.data["grid"][r][c].as_str().unwrap_or("").into();
            if g.found.iter().any(|w| coords(&w["cells"]).contains(&p)) {
                color = a;
                ink = bg;
            }
        }
        "sliding-puzzle" => {
            if value > 0 {
                label = value.to_string();
                color = a.darker(0.4);
            } else {
                color = bg.darker(0.03);
            }
        }
        "lights-out" => {
            label = if value == 1 { "ON" } else { "off" }.into();
            if value == 1 {
                color = a;
                ink = bg;
            } else {
                color = bg.lighter(0.035);
                ink = fg.with_alpha(0.55);
            }
        }
        "minesweeper" => {
            if g.revealed.contains(&p) {
                color = bg.lighter(0.025);
                if g.mine(p) {
                    label = "*".into();
                    color = Color::srgb(0.7, 0.25, 0.15);
                } else {
                    let n = g.adjacent(p);
                    if n > 0 {
                        label = n.to_string();
                        ink = a;
                    }
                }
            } else if g.flags.contains(&p) {
                label = "F".into();
                color = a.darker(0.4);
            } else {
                color = bg.lighter(0.23);
                if g.lost && g.mine(p) {
                    label = "*".into();
                }
            }
        }
        _ => {}
    }
    if g.selected == Some(p) {
        color = a;
        ink = bg;
    }
    (label, color, ink)
}
fn controls(
    c: &mut Commands,
    parent: Entity,
    g: &Game,
    top: f32,
    a: Color,
    fg: Color,
    bg: Color,
) -> f32 {
    let panel = bg.lighter(0.1);
    match g.puzzle.kind.as_str() {
        "sudoku" | "mini-sudoku" => {
            for n in 1..=g.cols() {
                let j = n - 1;
                button(
                    c,
                    parent,
                    n.to_string(),
                    [
                        22. + (j % 5) as f32 * 88.,
                        top + (j / 5) as f32 * 58.,
                        80.,
                        52.,
                    ],
                    Action::Enter(n.to_string()),
                    panel,
                    a,
                    29.,
                );
            }
            button(
                c,
                parent,
                "Erase",
                [374., top + 58., 84., 52.],
                Action::Enter("0".into()),
                panel,
                fg,
                21.,
            );
            top + 116.
        }
        "arithmetic" => {
            for (j, s) in [
                "1", "2", "3", "4", "5", "6", "7", "8", "9", "-", "0", "erase",
            ]
            .into_iter()
            .enumerate()
            {
                button(
                    c,
                    parent,
                    if s == "erase" { "Erase" } else { s },
                    [
                        22. + (j % 3) as f32 * 148.,
                        top + (j / 3) as f32 * 62.,
                        140.,
                        55.,
                    ],
                    Action::Enter(s.into()),
                    panel,
                    a,
                    29.,
                );
            }
            top + 248.
        }
        "cryptogram" => {
            for (j, ch) in ('A'..='Z').enumerate() {
                button(
                    c,
                    parent,
                    ch.to_string(),
                    [
                        22. + (j % 7) as f32 * 63.,
                        top + (j / 7) as f32 * 58.,
                        57.,
                        52.,
                    ],
                    Action::Enter(ch.to_string()),
                    panel,
                    a,
                    26.,
                );
            }
            button(
                c,
                parent,
                "Erase",
                [337., top + 174., 120., 52.],
                Action::Enter(String::new()),
                panel,
                fg,
                21.,
            );
            top + 232.
        }
        "maze" => {
            for (j, (d, label)) in [
                ("left", "Left"),
                ("up", "Up"),
                ("down", "Down"),
                ("right", "Right"),
            ]
            .into_iter()
            .enumerate()
            {
                button(
                    c,
                    parent,
                    label,
                    [22. + j as f32 * 110., top, 104., 56.],
                    Action::Step(d),
                    panel,
                    a,
                    23.,
                );
            }
            top + 56.
        }
        "nonogram" | "minesweeper" => {
            let mine = g.puzzle.kind == "minesweeper";
            button(
                c,
                parent,
                if mine { "Reveal" } else { "Paint" },
                [22., top, 210., 54.],
                Action::Mode(false),
                if !g.alternate { a } else { panel },
                if !g.alternate { bg } else { fg },
                23.,
            );
            button(
                c,
                parent,
                if mine { "Flag" } else { "Cross" },
                [248., top, 210., 54.],
                Action::Mode(true),
                if g.alternate { a } else { panel },
                if g.alternate { bg } else { fg },
                23.,
            );
            if mine {
                text(
                    c,
                    parent,
                    format!(
                        "{} flags / {} mines",
                        g.flags.len(),
                        g.puzzle.data["mine_count"]
                    ),
                    24.,
                    top + 62.,
                    430.,
                    19.,
                    fg,
                );
            }
            top + if mine { 94. } else { 54. }
        }
        "word-search" => {
            let words = g.puzzle.data["words"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let line = words
                .iter()
                .map(|w| {
                    format!(
                        "{}{}",
                        if g.found.iter().any(|f| f["word"] == *w) {
                            "[OK] "
                        } else {
                            ""
                        },
                        w.as_str().unwrap_or("")
                    )
                })
                .collect::<Vec<_>>()
                .join("   ");
            text(c, parent, line, 24., top, 430., 22., a);
            top + 110.
        }
        _ => top,
    }
}
