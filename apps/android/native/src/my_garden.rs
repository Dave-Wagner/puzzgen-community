//! First sanctuary checkpoint: an enjoy/arrange garden with earned lanterns.
use super::*;
use home::{HomeArt, label, node, pill};
const INK: Color = Color::srgb(0.055, 0.20, 0.14);
const CREAM: Color = Color::srgb(0.99, 0.94, 0.81);
const SLOTS: [(f32, f32); 8] = [
    (-2., -2.),
    (0., -2.),
    (2., -2.),
    (-2., 0.),
    (2., 0.),
    (-2., 2.),
    (0., 2.),
    (2., 2.),
];
#[derive(Component)]
pub(super) struct HomeWorld;
#[derive(Component)]
pub(super) struct HomeEyes;
#[derive(Component)]
pub(super) struct Placed;
#[derive(Component)]
pub(super) struct Spot;
#[derive(Component)]
pub(super) struct Heart;
#[derive(Component)]
pub(super) struct Mote(f32);

pub(super) fn hide_maze(
    p: Res<Pocket>,
    mut worlds: Query<&mut Visibility, With<garden_walk::World>>,
) {
    for mut visibility in &mut worlds {
        *visibility = if garden_walk::active(&p) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
fn mat(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    unlit: bool,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        unlit,
        perceptual_roughness: 0.9,
        ..default()
    })
}
fn plant(
    c: &mut Commands,
    assets: &AssetServer,
    root: Entity,
    name: &str,
    x: f32,
    z: f32,
    size: f32,
) {
    c.spawn((
        garden_weather::PendingPlant(assets.load(format!("art/garden3d/{name}.gltf#Scene0"))),
        Transform::from_xyz(x, 0., z).with_scale(Vec3::splat(size)),
        ChildOf(root),
    ));
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn update(
    mut c: Commands,
    mut p: ResMut<Pocket>,
    canvas: Res<Canvas>,
    time: Res<Time>,
    assets: Res<AssetServer>,
    windows: Query<&Window>,
    touches: Res<Touches>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut worlds: Query<(Entity, &mut Visibility), (With<HomeWorld>, Without<Spot>)>,
    mut cameras: Query<(&mut Camera, &Transform), With<HomeEyes>>,
    mut spots: Query<&mut Visibility, (With<Spot>, Without<HomeWorld>)>,
    placed: Query<Entity, With<Placed>>,
    mut hearts: Query<&mut Transform, (With<Heart>, Without<Mote>, Without<HomeEyes>)>,
    mut motes: Query<(&Mote, &mut Transform), (Without<Heart>, Without<HomeEyes>)>,
    state: (Local<Option<[bool; 8]>>, ResMut<AmbientLight>),
) {
    let (mut last_layout, mut ambient) = state;
    let active = p.page == Page::MyGarden;
    for (_, mut v) in &mut worlds {
        *v = if active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (mut camera, _) in &mut cameras {
        camera.is_active = active;
    }
    if !active {
        return;
    }
    for mut v in &mut spots {
        *v = if p.arranging {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    ambient.color = Color::srgb(1., 0.94, 0.82);
    ambient.brightness = 550.;
    let root = if let Some((root, _)) = worlds.iter_mut().next() {
        root
    } else {
        let root = c
            .spawn((HomeWorld, Transform::default(), Visibility::default()))
            .id();
        let earth = mat(&mut materials, Color::srgb(0.15, 0.24, 0.10), false);
        c.spawn((
            Mesh3d(meshes.add(Cylinder::new(4.7, 0.18).mesh().resolution(48))),
            MeshMaterial3d(earth),
            Transform::from_xyz(0., -0.12, 0.),
            ChildOf(root),
        ));
        let stone = mat(&mut materials, Color::srgb(0.66, 0.61, 0.46), false);
        let disc = meshes.add(Cylinder::new(0.53, 0.05).mesh().resolution(10));
        for (x, z) in SLOTS {
            c.spawn((
                Mesh3d(disc.clone()),
                MeshMaterial3d(stone.clone()),
                Transform::from_xyz(x, 0., z),
                ChildOf(root),
            ));
        }
        for i in 0..12 {
            let angle = i as f32 * std::f32::consts::TAU / 12.;
            plant(
                &mut c,
                &assets,
                root,
                if i % 3 == 0 { "Bush_Flowers" } else { "Bush" },
                angle.cos() * 3.9,
                angle.sin() * 3.9,
                0.85,
            );
            plant(
                &mut c,
                &assets,
                root,
                "Flower_4_Clump",
                angle.cos() * 3.1,
                angle.sin() * 3.1,
                0.33,
            );
        }
        for (x, z) in [(-3.4, -3.4), (3.4, -3.4)] {
            plant(&mut c, &assets, root, "BirchTree_1", x, z, 3.5);
        }
        let gold = materials.add(StandardMaterial {
            base_color: Color::srgb(1., 0.79, 0.30),
            emissive: LinearRgba::rgb(3., 1.7, 0.2),
            unlit: true,
            ..default()
        });
        c.spawn((
            Mesh3d(meshes.add(Sphere::new(0.24))),
            MeshMaterial3d(gold.clone()),
            Transform::from_xyz(0., 1.5, 0.),
            Heart,
            garden_weather::Special,
            ChildOf(root),
        ));
        let mote = meshes.add(Sphere::new(0.035).mesh().ico(1).expect("mote"));
        for i in 0..24 {
            c.spawn((
                Mesh3d(mote.clone()),
                MeshMaterial3d(gold.clone()),
                Transform::default(),
                Mote(i as f32),
                garden_weather::Special,
                ChildOf(root),
            ));
        }
        let marker = mat(&mut materials, Color::srgb(0.45, 0.80, 0.47), true);
        let circle = meshes.add(Cylinder::new(0.44, 0.018).mesh().resolution(20));
        for (x, z) in SLOTS {
            c.spawn((
                Mesh3d(circle.clone()),
                MeshMaterial3d(marker.clone()),
                Transform::from_xyz(x, 0.04, z),
                Visibility::Hidden,
                Spot,
                garden_weather::Special,
                ChildOf(root),
            ));
        }
        c.spawn((
            DirectionalLight {
                illuminance: 4500.,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.7, 0.)),
            ChildOf(root),
        ));
        c.spawn((
            Camera3d::default(),
            Camera {
                order: 0,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.08, 0.15, 0.12)),
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: 50_f32.to_radians(),
                ..default()
            }),
            bevy::core_pipeline::tonemapping::Tonemapping::Reinhard,
            Msaa::Off,
            Transform::from_xyz(7.8, 9.8, 11.5).looking_at(Vec3::new(0., 0.4, 0.), Vec3::Y),
            HomeEyes,
            ChildOf(root),
        ));
        *last_layout = None;
        root
    };
    // Camera pixels and tap rays use the same safe-area canvas; UI stays outside this viewport.
    if let Ok(window) = windows.single() {
        let factor = window.scale_factor();
        for (mut camera, _) in &mut cameras {
            camera.viewport = Some(bevy::camera::Viewport {
                physical_position: UVec2::new(
                    (canvas.offset.x * factor).max(0.) as u32,
                    ((canvas.offset.y + 180. * canvas.scale) * factor).max(0.) as u32,
                ),
                physical_size: UVec2::new(
                    (480. * canvas.scale * factor).max(1.) as u32,
                    ((canvas.height - 400.).max(100.) * canvas.scale * factor).max(1.) as u32,
                ),
                ..default()
            });
        }
    }
    if last_layout.as_ref() != Some(&p.saved.sanctuary.layout) {
        for entity in &placed {
            c.entity(entity).despawn();
        }
        for (i, &present) in p.saved.sanctuary.layout.iter().enumerate() {
            if present {
                let (x, z) = SLOTS[i];
                c.spawn((
                    garden_weather::PendingPlant(
                        assets.load("art/garden3d/ornaments/Lantern.glb#Scene0"),
                    ),
                    Transform::from_xyz(x, 0.035, z).with_scale(Vec3::splat(1.3)),
                    Placed,
                    ChildOf(root),
                ));
            }
        }
        *last_layout = Some(p.saved.sanctuary.layout);
    }
    let t = if p.saved.reduced_motion {
        0.
    } else {
        time.elapsed_secs()
    };
    for mut transform in &mut hearts {
        transform.scale = Vec3::splat(1. + (t * 1.1).sin() * 0.10);
    }
    for (m, mut transform) in &mut motes {
        let phase = m.0 * 2.399 + t * 0.22;
        transform.translation = Vec3::new(
            phase.cos() * (0.5 + m.0 / 50.),
            0.5 + m.0 / 16. + (phase * 1.4).sin() * 0.10,
            phase.sin() * (0.5 + m.0 / 50.),
        );
    }
    if !p.arranging || !windows.single().is_ok_and(|w| w.focused) {
        return;
    }
    for touch in touches.iter_just_released() {
        if touch.position().distance_squared(touch.start_position()) > 64. {
            continue;
        }
        let point = (touch.position() - canvas.offset) / canvas.scale;
        if point.y < 180. || point.y > canvas.height - 220. {
            continue;
        }
        let Some((camera, transform)) = cameras.iter_mut().next() else {
            continue;
        };
        let Ok(ray) =
            camera.viewport_to_world(&GlobalTransform::from(*transform), touch.position())
        else {
            continue;
        };
        if ray.direction.y >= -0.01 {
            continue;
        }
        let hit = ray.get_point(-ray.origin.y / ray.direction.y);
        let chosen = SLOTS
            .iter()
            .enumerate()
            .find(|(_, s)| {
                (Vec2::new(s.0, s.1) - Vec2::new(hit.x, hit.z)).length_squared() < 0.55 * 0.55
            })
            .map(|(i, _)| i);
        if let Some(spot) = chosen {
            let before = p.saved.sanctuary.layout;
            match p.saved.sanctuary.toggle(spot) {
                Ok(()) => {
                    if p.garden_history.len() == 20 {
                        p.garden_history.remove(0);
                    }
                    p.garden_history.push(before);
                    p.message = "A little more of you, here at home.".into();
                }
                Err(message) => p.message = message.into(),
            }
            p.dirty = true;
            persist(&p);
        }
    }
}

pub(super) fn draw(c: &mut Commands, root: Entity, p: &mut Pocket, canvas: &Canvas, art: &HomeArt) {
    let h = canvas.height;
    for rect in [[0., 0., 480., 180.], [0., h - 220., 480., 220.]] {
        c.spawn((
            node(rect),
            BackgroundColor(INK),
            ChildOf(root),
            bevy::ui::FocusPolicy::Pass,
        ));
    }
    pill(
        c,
        root,
        "<",
        [18., 20., 55., 52.],
        Action::Home,
        CREAM,
        INK,
        &art.body,
    );
    beacon::draw(c, root, p, art, [410., 20., 52., 52.]);
    label(
        c,
        root,
        "My Garden",
        [80., 22., 320., 48.],
        34.,
        CREAM,
        &art.display,
        true,
    );
    label(
        c,
        root,
        "You belong here.",
        [45., 78., 390., 30.],
        22.,
        CREAM,
        &art.body,
        true,
    );
    label(
        c,
        root,
        format!(
            "{} Care points · {} lanterns to place",
            p.saved.sanctuary.care_points,
            p.saved.sanctuary.available()
        ),
        [22., 126., 436., 34.],
        20.,
        CREAM,
        &art.body,
        true,
    );
    label(
        c,
        root,
        if p.arranging {
            "Tap a glowing spot to place or lift a lantern."
        } else {
            "Rest a moment. Your garden is always here."
        },
        [24., h - 208., 432., 48.],
        20.,
        CREAM,
        &art.body,
        true,
    );
    label(
        c,
        root,
        &p.message,
        [24., h - 155., 432., 55.],
        18.,
        CREAM,
        &art.body,
        true,
    );
    pill(
        c,
        root,
        if p.arranging {
            "Enjoy garden"
        } else {
            "Arrange garden"
        },
        [24., h - 83., 242., 58.],
        Action::GardenArrange,
        CREAM,
        INK,
        &art.body,
    );
    if p.arranging && !p.garden_history.is_empty() {
        pill(
            c,
            root,
            "Undo",
            [286., h - 83., 170., 58.],
            Action::GardenUndo,
            CREAM,
            INK,
            &art.body,
        );
    }
    p.content_height = h;
    p.scroll = 0.;
}
