//! A shared, gentle home companion across the sanctuary and puzzle pages.
use super::*;
use home::{HomeArt, node};
#[derive(Component)]
pub(super) struct Heart(bool);
#[derive(Component)]
pub(super) struct Wisp {
    phase: f32,
    radius: f32,
}
pub(super) fn draw(c: &mut Commands, root: Entity, p: &Pocket, art: &HomeArt, rect: [f32; 4]) {
    let gold = Color::srgb(1., 0.79, 0.35);
    let emerald = Color::srgb(0.30, 0.78, 0.48);
    let caption = rect[3] > 40.;
    let center = Vec2::new(rect[2] * 0.5, if caption { 20. } else { rect[3] * 0.5 });
    let owner = c
        .spawn((
            Button,
            Action::MyGarden,
            HitRect(rect),
            Node {
                border: UiRect::all(px(1.)),
                ..node(rect)
            },
            BackgroundColor(Color::srgb(0.055, 0.20, 0.14)),
            BorderRadius::all(px(26.)),
            BorderColor::all(gold.with_alpha(0.8)),
            BoxShadow(vec![ShadowStyle {
                color: gold.with_alpha(0.28),
                blur_radius: px(10.),
                spread_radius: px(2.),
                ..default()
            }]),
            ChildOf(root),
        ))
        .id();
    c.spawn((
        node([center.x - 9., center.y - 9., 18., 18.]),
        BackgroundColor(Color::srgb(1., 0.94, 0.70)),
        BorderRadius::all(px(12.)),
        UiTransform::IDENTITY,
        Heart(p.page == Page::Play && p.saved.game.as_ref().is_some_and(|g| g.won)),
        BoxShadow(vec![ShadowStyle {
            color: gold.with_alpha(0.85),
            blur_radius: px(9.),
            spread_radius: px(2.),
            ..default()
        }]),
        ChildOf(owner),
        bevy::ui::FocusPolicy::Pass,
    ));
    for i in 0..3 {
        c.spawn((
            ImageNode {
                image: art.sparkle.clone(),
                color: if i == 1 { emerald } else { gold },
                ..default()
            },
            node([center.x - 2.5, center.y - 2.5, 5., 5.]),
            UiTransform::IDENTITY,
            Wisp {
                phase: i as f32 * 2.094,
                radius: 14.,
            },
            ChildOf(owner),
            bevy::ui::FocusPolicy::Pass,
        ));
    }
    if caption {
        home::label(
            c,
            owner,
            "Home",
            [0., 35., rect[2], 14.],
            11.,
            Color::srgb(1., 0.94, 0.81),
            &art.body,
            true,
        );
    }
}
pub(super) fn animate(
    time: Res<Time>,
    p: Res<Pocket>,
    mut hearts: Query<(&Heart, &mut UiTransform)>,
    mut wisps: Query<(&Wisp, &mut UiTransform), Without<Heart>>,
) {
    let t = if p.saved.reduced_motion {
        0.
    } else {
        time.elapsed_secs()
    };
    for (heart, mut transform) in &mut hearts {
        transform.scale = Vec2::splat(if p.saved.reduced_motion {
            1.
        } else {
            1. + (t * if heart.0 { 1.6 } else { 0.9 }).sin() * if heart.0 { 0.16 } else { 0.07 }
        });
    }
    for (wisp, mut transform) in &mut wisps {
        let angle = t * 0.35 + wisp.phase;
        transform.translation =
            bevy::ui::Val2::px(angle.cos() * wisp.radius, angle.sin() * wisp.radius * 0.55);
    }
}
