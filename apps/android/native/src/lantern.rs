//! Minimal overlay over the native first-person garden; no direction-pad UI.
use super::*;
use home::{HomeArt, label, node, pill};
const CREAM: Color = Color::srgb(0.99, 0.94, 0.81);
const INK: Color = Color::srgb(0.055, 0.20, 0.14);
pub(super) fn draw(
    c: &mut Commands,
    root: Entity,
    p: &mut Pocket,
    canvas: &Canvas,
    art: &HomeArt,
    feed: &lost_lantern::Feed,
) {
    let Some(g) = p.saved.game.as_ref() else {
        return;
    };
    let h = canvas.height;
    c.spawn((
        node([0., 0., 480., 120.]),
        BackgroundColor(INK.with_alpha(0.82)),
        ChildOf(root),
        bevy::ui::FocusPolicy::Pass,
    ));
    beacon::draw(c, root, p, art, [18., 20., 55., 52.]);
    label(
        c,
        root,
        "The Lost Lantern",
        [78., 24., 324., 46.],
        31.,
        CREAM,
        &art.display,
        true,
    );
    pill(
        c,
        root,
        if p.saved.muted { "Off" } else { "On" },
        [406., 20., 57., 52.],
        Action::Mute,
        CREAM,
        INK,
        &art.body,
    );
    label(
        c,
        root,
        if p.busy {
            "Opening the garden…"
        } else {
            "Follow the light. Bring a lantern home."
        },
        [30., 80., 420., 32.],
        21.,
        CREAM,
        &art.body,
        true,
    );
    pill(
        c,
        root,
        "Weather",
        [328., 128., 134., 44.],
        Action::Weather,
        INK.with_alpha(0.85),
        CREAM,
        &art.body,
    );
    for (mode, title, x) in [
        (garden_walk::ControlMode::Hold, "Hold to walk", 18.),
        (garden_walk::ControlMode::Tap, "Tap to walk", 175.),
    ] {
        let selected = p.saved.garden_control == mode;
        pill(
            c,
            root,
            title,
            [x, 128., 145., 44.],
            Action::GardenControl(mode),
            if selected {
                CREAM
            } else {
                INK.with_alpha(0.85)
            },
            if selected { INK } else { CREAM },
            &art.body,
        );
    }
    if !g.won {
        lost_lantern::hud(c, root, p, feed, art);
    }
    let pad = garden_walk::pad_rect(h, p.saved.garden_left_handed);
    c.spawn((
        Node {
            border: UiRect::all(px(2.)),
            ..node(pad)
        },
        BackgroundColor(INK),
        BorderColor::all(CREAM.with_alpha(0.8)),
        BorderRadius::all(px(28.)),
        garden_walk::WalkPad,
        ChildOf(root),
        bevy::ui::FocusPolicy::Pass,
    ));
    let hold = p.saved.garden_control == garden_walk::ControlMode::Hold;
    label(
        c,
        root,
        if hold {
            "WALK\nHold to move"
        } else {
            "STOP\nTap to stop"
        },
        [pad[0], pad[1] + 12., pad[2], 64.],
        23.,
        CREAM,
        &art.body,
        true,
    );
    let hint_x = if p.saved.garden_left_handed {
        240.
    } else {
        18.
    };
    c.spawn((
        node([hint_x, h - 170., 222., 75.]),
        BackgroundColor(INK.with_alpha(0.85)),
        BorderRadius::all(px(16.)),
        ChildOf(root),
        bevy::ui::FocusPolicy::Pass,
    ));
    c.spawn((
        Text::new(""),
        TextFont {
            font: art.body.clone(),
            font_size: 17.,
            ..default()
        },
        TextColor(CREAM),
        TextLayout::new_with_justify(Justify::Center),
        node([hint_x + 6., h - 158., 210., 62.]),
        garden_walk::ControlHint,
        ChildOf(root),
        bevy::ui::FocusPolicy::Pass,
    ));
    pill(
        c,
        root,
        if p.saved.garden_left_handed {
            "Hand: Left"
        } else {
            "Hand: Right"
        },
        [18., h - 65., 190., 46.],
        Action::GardenHand,
        INK.with_alpha(0.85),
        CREAM,
        &art.body,
    );
    if g.won {
        c.spawn((
            node([28., h * 0.32, 424., 290.]),
            BackgroundColor(INK.with_alpha(0.92)),
            BorderRadius::all(px(20.)),
            ChildOf(root),
        ));
        label(
            c,
            root,
            "Welcome home",
            [42., h * 0.32 + 18., 396., 50.],
            30.,
            CREAM,
            &art.display,
            true,
        );
        label(
            c,
            root,
            &p.message,
            [48., h * 0.32 + 76., 384., 65.],
            20.,
            CREAM,
            &art.body,
            true,
        );
        pill(
            c,
            root,
            "Visit My Garden",
            [92., h * 0.32 + 150., 296., 54.],
            Action::MyGarden,
            CREAM,
            INK,
            &art.display,
        );
        pill(
            c,
            root,
            "Another garden",
            [122., h * 0.32 + 216., 236., 50.],
            Action::New,
            CREAM,
            INK,
            &art.display,
        );
    } else {
        pill(
            c,
            root,
            "New garden",
            [310., h - 65., 150., 46.],
            Action::New,
            INK.with_alpha(0.8),
            CREAM,
            &art.body,
        );
    }
    p.content_height = h;
    p.scroll = 0.;
}
