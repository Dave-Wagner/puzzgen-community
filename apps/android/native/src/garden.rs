//! Native Sudoku pages. Mechanics remain in the shared model.
use super::*;
use bevy::ui::widget::NodeImageMode;
use home::{HomeArt, label, node, pill};
const INK: Color = Color::srgb(0.055, 0.20, 0.14);
const CREAM: Color = Color::srgb(0.99, 0.94, 0.81);
const GOLD: Color = Color::srgb(0.76, 0.59, 0.30);
#[derive(Component)]
pub(super) struct TilePop {
    age: f32,
}
#[derive(Component)]
pub(super) struct Mote {
    phase: f32,
    x: f32,
    y: f32,
}
pub(super) fn glints(c: &mut Commands, root: Entity, art: &HomeArt, won: bool) {
    for j in 0..if won { 28 } else { 10 } {
        let x = if j % 2 == 0 { 5. } else { 462. };
        let y = 130. + (j % 7) as f32 * 100.;
        c.spawn((
            ImageNode {
                image: art.sparkle.clone(),
                color: GOLD.with_alpha(0.45),
                ..default()
            },
            node([x, y, 12., 12.]),
            Mote {
                phase: j as f32 * 1.618,
                x,
                y,
            },
            ChildOf(root),
            bevy::ui::FocusPolicy::Pass,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn key(
    c: &mut Commands,
    parent: Entity,
    text: &str,
    r: [f32; 4],
    action: Option<Action>,
    font: &Handle<Font>,
    size: f32,
    bg: Color,
    fg: Color,
    selected: bool,
) -> Entity {
    let e = c
        .spawn((
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(if selected { 3. } else { 1. })),
                ..node(r)
            },
            BackgroundColor(bg),
            BorderColor::all(if selected {
                Color::srgb(0.27, 0.61, 0.27)
            } else {
                GOLD.with_alpha(0.85)
            }),
            BorderRadius::all(px(12.)),
            UiTransform::IDENTITY,
            BoxShadow(vec![ShadowStyle {
                color: Color::srgba(0.05, 0.10, 0.035, 0.3),
                x_offset: px(0.),
                y_offset: px(3.),
                spread_radius: px(0.),
                blur_radius: px(2.),
            }]),
            ChildOf(parent),
        ))
        .id();
    if let Some(action) = action {
        c.entity(e).insert((Button, action, HitRect(r)));
    }
    if selected {
        c.entity(e).insert(TilePop { age: 0. });
    }
    label(
        c,
        e,
        text,
        [0., (r[3] - size * 1.25) * 0.5, r[2], r[3]],
        size,
        fg,
        font,
        true,
    );
    e
}
pub(super) fn draw(c: &mut Commands, root: Entity, p: &mut Pocket, canvas: &Canvas, art: &HomeArt) {
    let Some(g) = p.saved.game.clone() else {
        return;
    };
    let h = canvas.height;
    let library = g.index() == 0;
    let n = if library { 9 } else { 4 };
    let group = if library { 3 } else { 2 };
    c.spawn((
        ImageNode {
            image: art.garden.clone(),
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        node([0., 0., 480., h]),
        ChildOf(root),
        bevy::ui::FocusPolicy::Pass,
    ));
    if library {
        c.spawn((
            ImageNode {
                image: art.worlds.clone(),
                rect: Some(Rect::from_corners(
                    Vec2::new(0., 100.),
                    Vec2::new(627., 360.),
                )),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            node([0., 0., 480., 124.]),
            ChildOf(root),
            bevy::ui::FocusPolicy::Pass,
        ));
        c.spawn((
            node([0., 0., 480., 124.]),
            BackgroundColor(INK.with_alpha(0.76)),
            ChildOf(root),
            bevy::ui::FocusPolicy::Pass,
        ));
    }
    if !p.saved.reduced_motion {
        for j in 0..if g.won { 28 } else { 10 } {
            let x = if j % 2 == 0 { 5. } else { 462. };
            let y = 130. + (j % 7) as f32 * 100.;
            c.spawn((
                ImageNode {
                    image: art.sparkle.clone(),
                    color: GOLD.with_alpha(0.45),
                    ..default()
                },
                node([x, y, 12., 12.]),
                Mote {
                    phase: j as f32 * 1.618,
                    x,
                    y,
                },
                ChildOf(root),
                bevy::ui::FocusPolicy::Pass,
            ));
        }
    }
    beacon::draw(c, root, p, art, [18., 25., 55., 52.]);
    label(
        c,
        root,
        if library {
            "Emerald Library"
        } else {
            "Toybox Garden"
        },
        [77., 27., 326., 45.],
        34.,
        CREAM,
        &art.display,
        true,
    );
    pill(
        c,
        root,
        if p.saved.muted { "Off" } else { "On" },
        [406., 25., 57., 52.],
        Action::Mute,
        CREAM,
        INK,
        &art.body,
    );
    label(
        c,
        root,
        "Sound",
        [406., 80., 57., 24.],
        14.,
        CREAM,
        &art.body,
        true,
    );
    label(
        c,
        root,
        if library {
            "Sudoku · A quiet chapter"
        } else {
            "Mini Sudoku"
        },
        [78., 78., 324., 31.],
        21.,
        CREAM,
        &art.body,
        true,
    );
    let top = (h * 0.17).clamp(130., 190.);
    let size =
        (h - if library { 535. } else { 465. }).clamp(if library { 280. } else { 300. }, 440.);
    let left = (480. - size) * 0.5;
    label(
        c,
        root,
        if library {
            "Rows, columns and boxes use 1–9."
        } else {
            "Rows, columns and boxes use 1–4."
        },
        [24., top - 43., 432., 30.],
        21.,
        CREAM,
        &art.body,
        true,
    );
    c.spawn((
        Node {
            border: UiRect::all(px(3.)),
            ..node([left, top, size, size])
        },
        BackgroundColor(if library {
            Color::srgb(0.22, 0.16, 0.09)
        } else {
            INK
        }),
        BorderColor::all(GOLD),
        BorderRadius::all(px(23.)),
        BoxShadow(vec![ShadowStyle {
            color: Color::srgba(0.02, 0.06, 0.025, 0.50),
            x_offset: px(0.),
            y_offset: px(8.),
            spread_radius: px(1.),
            blur_radius: px(12.),
        }]),
        ChildOf(root),
    ));
    let pad = 13.;
    let inner = size - pad * 2.;
    let gap = if library { 2. } else { 4. };
    let box_gap = if library { 7. } else { 10. };
    let tile = (inner - gap * (n - 1) as f32 - (box_gap - gap) * (n / group - 1) as f32) / n as f32;
    for r in 0..n {
        for col in 0..n {
            let x = left + pad + col as f32 * (tile + gap) + (col / group) as f32 * (box_gap - gap);
            let y = top + pad + r as f32 * (tile + gap) + (r / group) as f32 * (box_gap - gap);
            let value = g.grid[r][col];
            let given = g.puzzle.data["grid"][r][col].as_i64().unwrap_or(0) != 0;
            let selected = g.selected == Some([r, col]);
            let text = if value == 0 {
                String::new()
            } else {
                value.to_string()
            };
            let bg = if given {
                Color::srgb(0.94, 0.87, 0.69)
            } else {
                CREAM
            };
            key(
                c,
                root,
                &text,
                [x, y, tile, tile],
                if g.won {
                    None
                } else {
                    Some(Action::Tap([r, col]))
                },
                &art.display,
                (tile * 0.53).clamp(if library { 20. } else { 33. }, 52.),
                bg,
                if given {
                    INK
                } else {
                    Color::srgb(0.14, 0.43, 0.23)
                },
                selected,
            );
        }
    }
    let keys_y = top + size + 18.;
    let key_h = if library {
        49.
    } else if h < 900. {
        60.
    } else {
        68.
    };
    let can_enter = !g.won
        && !p.busy
        && g.selected
            .is_some_and(|[r, col]| g.puzzle.data["grid"][r][col].as_i64().unwrap_or(0) == 0);
    let digits: &[&str] = if library {
        &["1", "2", "3", "4", "5", "6", "7", "8", "9", "Erase"]
    } else {
        &["1", "2", "3", "4", "Erase"]
    };
    for (j, &t) in digits.iter().enumerate() {
        let enabled = can_enter
            && g.selected
                .is_some_and(|[r, col]| g.grid[r][col] != if j == n { 0 } else { j as i32 + 1 });
        key(
            c,
            root,
            t,
            [
                22. + (j % 5) as f32 * 88.,
                keys_y + (j / 5) as f32 * (key_h + 7.),
                84.,
                key_h,
            ],
            enabled.then(|| Action::Enter(if j == n { "0".into() } else { t.into() })),
            &art.display,
            if j == n {
                22.
            } else if library {
                30.
            } else {
                38.
            },
            CREAM,
            if enabled { INK } else { INK.with_alpha(0.4) },
            false,
        );
    }
    let action_y = keys_y + key_h + if library { key_h + 27. } else { 20. };
    let small_h = 56.;
    key(
        c,
        root,
        "Undo",
        [22., action_y, 101., small_h],
        (!p.history.is_empty() && !p.busy && !g.won).then_some(Action::Undo),
        &art.body,
        22.,
        CREAM,
        if !p.history.is_empty() && !p.busy && !g.won {
            INK
        } else {
            INK.with_alpha(0.4)
        },
        false,
    );
    key(
        c,
        root,
        "Hint",
        [134., action_y, 101., small_h],
        (!p.busy && !g.won).then_some(Action::Hint),
        &art.body,
        22.,
        CREAM,
        if !p.busy && !g.won {
            INK
        } else {
            INK.with_alpha(0.4)
        },
        false,
    );
    label(
        c,
        root,
        if g.won {
            "Lovely work!".to_string()
        } else if p.busy {
            "Please wait…".to_string()
        } else {
            format!(
                "{} squares left",
                g.grid.iter().flatten().filter(|&&v| v == 0).count()
            )
        },
        [247., action_y + 12., 211., small_h],
        24.,
        CREAM,
        &art.display,
        true,
    );
    let new_y = action_y + 79.;
    pill(
        c,
        root,
        "New puzzle",
        [if g.won { 246. } else { 134. }, new_y, 212., 52.],
        Action::New,
        CREAM,
        INK,
        &art.display,
    );
    if g.won {
        pill(
            c,
            root,
            "My Garden",
            [22., new_y, 212., 52.],
            Action::MyGarden,
            CREAM,
            INK,
            &art.display,
        );
    }
    let tip = if !g.won && g.grid.iter().flatten().all(|&v| v != 0) {
        "A number repeats in a row, column or box. You can change any number you placed."
    } else if !p.message.is_empty() {
        &p.message
    } else if g.won {
        "Beautifully done. Another little adventure is waiting."
    } else if g
        .selected
        .is_some_and(|[r, c]| g.puzzle.data["grid"][r][c].as_i64().unwrap_or(0) != 0)
    {
        "This starting number stays fixed. Choose a blank square to add your own."
    } else if g.selected.is_some() {
        "Choose a number below, or Erase to try again."
    } else {
        "Choose an empty square, then a number."
    };
    let message_y = new_y + 68.;
    c.spawn((
        node([22., message_y - 4., 436., 84.]),
        BackgroundColor(INK.with_alpha(0.80)),
        BorderRadius::all(px(13.)),
        ChildOf(root),
        bevy::ui::FocusPolicy::Pass,
    ));
    label(
        c,
        root,
        tip,
        [34., message_y + 4., 412., 76.],
        21.,
        CREAM,
        &art.body,
        true,
    );
    p.content_height = h.max(message_y + 100.);
    p.scroll = p.scroll.clamp(0., (p.content_height - h).max(0.));
}
pub(super) fn animate(
    time: Res<Time>,
    p: Res<Pocket>,
    mut tiles: Query<(&mut TilePop, &mut UiTransform)>,
    mut motes: Query<(&Mote, &mut Node, &mut ImageNode)>,
) {
    for (mut pop, mut transform) in &mut tiles {
        pop.age += time.delta_secs();
        let t = (pop.age / 0.22).min(1.);
        transform.scale = Vec2::splat(if p.saved.reduced_motion {
            1.
        } else {
            1. - 0.035 * (1. - t) * (t * std::f32::consts::PI).cos()
        });
    }
    for (mote, mut node, mut image) in &mut motes {
        let t = time.elapsed_secs() * 0.7 + mote.phase;
        node.left = px(mote.x + t.sin() * 3.);
        node.top = px(mote.y + t.cos() * 14.);
        image.color = GOLD.with_alpha(if p.saved.reduced_motion {
            0.
        } else {
            0.24 + 0.20 * (t * 0.8).sin().abs()
        });
    }
}
