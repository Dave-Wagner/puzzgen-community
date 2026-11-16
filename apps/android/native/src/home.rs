//! First page in the visual review. Native controls over reusable environment art.
use super::*;
use bevy::{color::Luminance, ui::widget::NodeImageMode};

#[derive(Resource)]
pub(super) struct HomeArt {
    pub worlds: Handle<Image>,
    pub welcome: Handle<Image>,
    pub garden: Handle<Image>,
    pub sparkle: Handle<Image>,
    pub display: Handle<Font>,
    pub body: Handle<Font>,
}
#[derive(Component)]
pub(super) struct Breathe {
    phase: f32,
}
pub(super) fn load(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(HomeArt {
        worlds: assets.load("art/home/worlds-v1.png"),
        welcome: assets.load("art/home/welcome-v1.png"),
        garden: assets.load("art/home/garden-v1.png"),
        sparkle: assets.load("art/star.png"),
        display: assets.load("fonts/BreeSerif-Regular.ttf"),
        body: assets.load("fonts/AtkinsonHyperlegible-Regular.ttf"),
    });
}
const INK: Color = Color::srgb(0.055, 0.20, 0.14);
const CREAM: Color = Color::srgb(0.99, 0.95, 0.85);
const GOLD: Color = Color::srgb(0.72, 0.54, 0.26);

pub(super) fn node(r: [f32; 4]) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(r[0]),
        top: px(r[1]),
        width: px(r[2]),
        height: px(r[3]),
        ..default()
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn label(
    c: &mut Commands,
    parent: Entity,
    s: impl Into<String>,
    r: [f32; 4],
    size: f32,
    color: Color,
    font: &Handle<Font>,
    center: bool,
) {
    c.spawn((
        Text::new(s),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextLayout::new_with_justify(if center {
            Justify::Center
        } else {
            Justify::Left
        }),
        node(r),
        ChildOf(parent),
        bevy::ui::FocusPolicy::Pass,
    ));
}
#[allow(clippy::too_many_arguments)]
pub(super) fn pill(
    c: &mut Commands,
    parent: Entity,
    s: &str,
    r: [f32; 4],
    action: Action,
    color: Color,
    text: Color,
    font: &Handle<Font>,
) {
    c.spawn((
        Button,
        action,
        HitRect(r),
        Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(1.)),
            ..node(r)
        },
        BackgroundColor(color),
        BorderColor::all(GOLD.with_alpha(0.65)),
        BorderRadius::all(px(r[3] * 0.5)),
        ChildOf(parent),
    ))
    .with_children(|c| {
        c.spawn((
            Text::new(s),
            TextFont {
                font: font.clone(),
                font_size: 21.,
                ..default()
            },
            TextColor(text),
        ));
    });
}
fn region(index: usize, aspect: f32) -> Rect {
    let half = 627.;
    let x = (index % 2) as f32 * half;
    let y = (index / 2) as f32 * half;
    let crop_h = (half / aspect).min(half);
    let crop_w = (half * aspect).min(half);
    Rect::from_corners(
        Vec2::new(x + (half - crop_w) * 0.5, y + (half - crop_h) * 0.5),
        Vec2::new(x + (half + crop_w) * 0.5, y + (half + crop_h) * 0.5),
    )
}
fn art_region(kind: usize) -> usize {
    match kind {
        0 | 2 | 8 => 0,
        1 | 3 | 4 | 7 => 1,
        5 | 6 => 2,
        _ => 3,
    }
}
pub(super) fn draw(c: &mut Commands, root: Entity, p: &mut Pocket, canvas: &Canvas, art: &HomeArt) {
    let h = canvas.height;
    c.spawn((
        node([0., 0., 480., h]),
        BackgroundColor(Color::srgb(0.985, 0.945, 0.84)),
        ChildOf(root),
        bevy::ui::FocusPolicy::Pass,
    ));
    let header = (h * 0.225).clamp(190., 250.);
    c.spawn((
        ImageNode {
            image: art.welcome.clone(),
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        node([0., 0., 480., header + 55.]),
        ChildOf(root),
        bevy::ui::FocusPolicy::Pass,
    ));
    label(
        c,
        root,
        "Beacon & Bough",
        [24., 32., 432., 57.],
        38.,
        INK,
        &art.display,
        true,
    );
    label(
        c,
        root,
        "Your beacon is here.",
        [76., 90., 380., 28.],
        22.,
        INK,
        &art.body,
        true,
    );
    beacon::draw(c, root, p, art, [26., 86., 38., 32.]);
    // The illustration is separate from text, hit regions and controls.
    let hero = [20., header - 26., 440., 164.];
    let has_game = p
        .saved
        .game
        .as_ref()
        .is_some_and(|g| matches!(g.index(), 0..=2 | 4));
    let hero_action = if has_game {
        Action::Open(p.saved.game.as_ref().map_or(1, Game::index))
    } else {
        Action::Open(1)
    };
    let hero_kind = if has_game {
        p.saved.game.as_ref().map_or(1, Game::index)
    } else {
        1
    };
    let hero_label = if has_game {
        "Continue playing"
    } else {
        "Let’s play"
    };
    let hero_entity = c
        .spawn((
            Button,
            hero_action,
            HitRect(hero),
            Node {
                overflow: Overflow::clip(),
                border: UiRect::all(px(2.)),
                ..node(hero)
            },
            BackgroundColor(INK),
            BorderColor::all(GOLD),
            BorderRadius::all(px(18.)),
            BoxShadow(vec![ShadowStyle {
                color: INK.with_alpha(0.16),
                x_offset: px(0.),
                y_offset: px(5.),
                spread_radius: px(0.),
                blur_radius: px(8.),
            }]),
            ChildOf(root),
        ))
        .id();
    c.spawn((
        ImageNode {
            image: art.worlds.clone(),
            rect: Some(region(art_region(hero_kind), hero[2] / hero[3])),
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        node([0., 0., hero[2], hero[3]]),
        UiTransform::IDENTITY,
        Breathe { phase: 0. },
        ChildOf(hero_entity),
        bevy::ui::FocusPolicy::Pass,
    ));
    c.spawn((
        node([0., 0., 300., hero[3]]),
        BackgroundColor(INK.with_alpha(0.82)),
        ChildOf(hero_entity),
        bevy::ui::FocusPolicy::Pass,
    ));
    label(
        c,
        hero_entity,
        hero_label,
        [20., 13., 285., 27.],
        18.,
        CREAM,
        &art.body,
        false,
    );
    label(
        c,
        hero_entity,
        TITLES[hero_kind],
        [20., 43., 370., 52.],
        29.,
        CREAM,
        &art.display,
        false,
    );
    // Entire hero is one action; the visual call-to-action does not create competing hit boxes.
    c.spawn((
        node([20., 112., 154., 39.]),
        BackgroundColor(CREAM),
        BorderRadius::all(px(20.)),
        ChildOf(hero_entity),
        bevy::ui::FocusPolicy::Pass,
    ));
    label(
        c,
        hero_entity,
        "Play now  >",
        [20., 116., 154., 30.],
        21.,
        INK,
        &art.display,
        true,
    );
    let heading_y = hero[1] + hero[3] + 21.;
    label(
        c,
        root,
        "Choose a little adventure",
        [22., heading_y, 365., 35.],
        25.,
        INK,
        &art.display,
        false,
    );
    let card_y = heading_y + 46.;
    let footer_y = h - 20.;
    let card_h = ((footer_y - card_y - 74.) / 2.).clamp(152., 225.);
    let gap = 14.;
    let image_h = card_h - 60.;
    let order = [0, 1, 4, 2];
    p.home_page = 0;
    for (j, &kind) in order.iter().skip(p.home_page * 4).take(4).enumerate() {
        let r = [
            20. + (j % 2) as f32 * 227.,
            card_y + (j / 2) as f32 * (card_h + gap),
            213.,
            card_h,
        ];
        let e = c
            .spawn((
                Button,
                Action::Open(kind),
                HitRect(r),
                Node {
                    overflow: Overflow::clip(),
                    border: UiRect::all(px(1.5)),
                    ..node(r)
                },
                BackgroundColor(CREAM),
                BorderColor::all(GOLD.with_alpha(0.75)),
                BorderRadius::all(px(14.)),
                BoxShadow(vec![ShadowStyle {
                    color: INK.with_alpha(0.12),
                    x_offset: px(0.),
                    y_offset: px(3.),
                    spread_radius: px(0.),
                    blur_radius: px(5.),
                }]),
                ChildOf(root),
            ))
            .id();
        if !matches!(kind, 0..=2 | 4) {
            c.entity(e).remove::<(Button, Action, HitRect)>();
        }
        c.spawn((
            ImageNode {
                image: art.worlds.clone(),
                rect: Some(region(art_region(kind), 213. / image_h)),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            node([0., 0., 213., image_h]),
            UiTransform::IDENTITY,
            Breathe {
                phase: j as f32 * 0.9,
            },
            ChildOf(e),
            bevy::ui::FocusPolicy::Pass,
        ));
        label(
            c,
            e,
            TITLES[kind],
            [6., image_h + 4., 201., 29.],
            21.,
            INK,
            &art.display,
            true,
        );
        label(
            c,
            e,
            if matches!(kind, 0..=2 | 4) {
                NAMES[kind]
            } else {
                "Coming later"
            },
            [6., image_h + 32., 201., 22.],
            16.,
            INK.lighter(0.12),
            &art.body,
            true,
        );
    }
    pill(
        c,
        root,
        "Credits",
        [350., 120., 110., 40.],
        Action::Credits,
        CREAM.with_alpha(0.88),
        INK,
        &art.body,
    );
    pill(
        c,
        root,
        "My Garden",
        [155., 120., 170., 40.],
        Action::MyGarden,
        CREAM.with_alpha(0.95),
        INK,
        &art.body,
    );
    pill(
        c,
        root,
        "Settings",
        [20., 120., 110., 40.],
        Action::Settings,
        CREAM.with_alpha(0.88),
        INK,
        &art.body,
    );
    p.content_height = h;
}
pub(super) fn animate(time: Res<Time>, p: Res<Pocket>, mut q: Query<(&Breathe, &mut UiTransform)>) {
    for (b, mut transform) in &mut q {
        let t = time.elapsed_secs() * 0.35 + b.phase;
        transform.scale = Vec2::splat(if p.saved.reduced_motion {
            1.
        } else {
            1.012 + t.sin() * 0.008
        });
    }
}
