//! First-person garden maze with two lightweight touch-control prototypes.
use super::*;
use bevy::core_pipeline::tonemapping::Tonemapping;
use walk_physics::TILE;

// A shared rounded mesh keeps the hedge silhouette soft without transparent leaf layers.
fn rounded_hedge() -> Mesh {
    use bevy::{
        asset::RenderAssetUsages, mesh::Indices, render::render_resource::PrimitiveTopology,
    };
    let half = Vec3::new(TILE / 2., 0.64, TILE / 2.);
    let radius = 0.28;
    let inner = half - Vec3::splat(radius);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    const STEPS: u32 = 8;
    for (n, u, v) in [
        (Vec3::X, Vec3::Y, Vec3::Z),
        (Vec3::NEG_X, Vec3::Y, Vec3::NEG_Z),
        (Vec3::Y, Vec3::Z, Vec3::X),
        (Vec3::NEG_Y, Vec3::Z, Vec3::NEG_X),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::NEG_Z, Vec3::X, Vec3::NEG_Y),
    ] {
        let base = positions.len() as u32;
        for y in 0..=STEPS {
            for x in 0..=STEPS {
                let uv = Vec2::new(x as f32, y as f32) / STEPS as f32;
                let cube = (n + u * (uv.x * 2. - 1.) + v * (uv.y * 2. - 1.)) * half;
                let nearest = cube.clamp(-inner, inner);
                let normal = (cube - nearest).normalize();
                positions.push((nearest + normal * radius).to_array());
                normals.push(normal.to_array());
                uvs.push(uv.to_array());
                if x < STEPS && y < STEPS {
                    let a = base + y * (STEPS + 1) + x;
                    indices.extend_from_slice(&[
                        a,
                        a + 1,
                        a + STEPS + 2,
                        a,
                        a + STEPS + 2,
                        a + STEPS + 1,
                    ]);
                }
            }
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}
#[derive(Component)]
pub(super) struct World;
#[derive(Component)]
pub(super) struct Eyes;
#[derive(Component)]
pub(super) struct WalkPad;
#[derive(Component)]
pub(super) struct ControlHint;
#[derive(Component)]
pub(super) struct Destination;
#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize)]
pub(super) enum ControlMode {
    #[default]
    Hold,
    Tap,
}
pub(super) fn pad_rect(height: f32, left_handed: bool) -> [f32; 4] {
    [
        if left_handed { 22. } else { 268. },
        height - 178.,
        190.,
        88.,
    ]
}
fn inside(point: Vec2, rect: [f32; 4]) -> bool {
    point.x >= rect[0]
        && point.y >= rect[1]
        && point.x < rect[0] + rect[2]
        && point.y < rect[1] + rect[3]
}
#[derive(Resource, Default)]
pub(super) struct Walk {
    key: String,
    pub position: Vec2,
    yaw: f32,
    pitch: f32,
    move_id: Option<u64>,
    look_id: Option<u64>,
    look_last: Vec2,
    look_start: Vec2,
    look_max_travel: f32,
    look_age: f32,
    target: Option<Vec2>,
    mode: ControlMode,
    left_handed: bool,
    invalid_tap_for: f32,
    pub stick: Vec2,
    pub walking: bool,
}
impl Walk {
    fn stop(&mut self) {
        self.move_id = None;
        self.look_id = None;
        self.target = None;
        self.stick = Vec2::ZERO;
        self.walking = false;
        self.invalid_tap_for = 0.;
    }
}
pub(super) fn active(p: &Pocket) -> bool {
    p.page == Page::Play && p.saved.game.as_ref().is_some_and(|g| g.index() == 2)
}
fn material(materials: &mut Assets<StandardMaterial>, color: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.95,
        ..default()
    })
}
#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    mut c: Commands,
    time: Res<Time>,
    mut p: ResMut<Pocket>,
    canvas: Res<Canvas>,
    mut walk: ResMut<Walk>,
    touches: Res<Touches>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    mut cameras: Query<(&mut Camera, &mut Transform), With<Eyes>>,
    mut overlays: Query<&mut Camera, (With<Camera2d>, Without<Eyes>)>,
    worlds: Query<Entity, With<World>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lifecycle: MessageReader<AppLifecycle>,
    assets: Res<AssetServer>,
    (mut atmosphere, lantern_feed): (ResMut<garden_weather::Atmosphere>, Res<lost_lantern::Feed>),
) {
    if lifecycle
        .read()
        .any(|event| matches!(event, AppLifecycle::Suspended))
    {
        walk.stop();
        return;
    }
    let enabled = active(&p);
    for mut camera in &mut overlays {
        camera.order = 1;
        camera.clear_color = if enabled || p.page == Page::MyGarden {
            ClearColorConfig::None
        } else {
            ClearColorConfig::Default
        };
    }
    for (mut camera, _) in &mut cameras {
        camera.is_active = enabled;
    }
    if !enabled {
        walk.stop();
        return;
    }
    if !windows.single().is_ok_and(|w| w.focused) || touches.iter_just_canceled().next().is_some() {
        walk.stop();
        return;
    }
    if walk.mode != p.saved.garden_control || walk.left_handed != p.saved.garden_left_handed {
        let entering_tap =
            walk.mode != p.saved.garden_control && p.saved.garden_control == ControlMode::Tap;
        walk.stop();
        walk.mode = p.saved.garden_control;
        walk.left_handed = p.saved.garden_left_handed;
        if entering_tap && walk.pitch > -0.20 {
            walk.pitch = -0.20;
        }
    }
    let Some(g) = p.saved.game.clone() else {
        return;
    };
    lost_lantern::prepare(&mut p, &g);
    let key = format!("{}:{}:{}", g.puzzle.seed, g.puzzle.difficulty, g.rows());
    if walk.key != key {
        for entity in &worlds {
            c.entity(entity).despawn();
        }
        walk.key = key;
        walk.stop();
        walk.pitch = if walk.mode == ControlMode::Tap {
            -0.20
        } else {
            0.
        };
        walk.walking = false;
        let [r, col] = g.path.last().copied().unwrap_or([1, 1]);
        walk.position = Vec2::new((col as f32 + 0.5) * TILE, (r as f32 + 0.5) * TILE);
        walk.yaw = 0.;
        for (dr, dc, yaw) in [
            (0, 1, -std::f32::consts::FRAC_PI_2),
            (1, 0, std::f32::consts::PI),
            (-1, 0, 0.),
            (0, -1, std::f32::consts::FRAC_PI_2),
        ] {
            let nr = r as isize + dr;
            let nc = col as isize + dc;
            if nr >= 0
                && nc >= 0
                && g.grid.get(nr as usize).and_then(|row| row.get(nc as usize)) == Some(&0)
            {
                walk.yaw = yaw;
                break;
            }
        }
        let root = c
            .spawn((World, Transform::default(), Visibility::default()))
            .id();
        lost_lantern::decorate(&mut c, root, &p, &mut meshes, &mut materials);
        let stones: Vec<_> = [
            Color::srgb(0.70, 0.66, 0.54),
            Color::srgb(0.76, 0.71, 0.60),
            Color::srgb(0.64, 0.62, 0.51),
        ]
        .into_iter()
        .map(|v| material(&mut materials, v))
        .collect();
        let hedge: Vec<_> = [
            Color::srgb(0.16, 0.32, 0.17),
            Color::srgb(0.17, 0.34, 0.18),
            Color::srgb(0.18, 0.35, 0.17),
        ]
        .into_iter()
        .map(|v| material(&mut materials, v))
        .collect();
        let earth = material(&mut materials, Color::srgb(0.18, 0.26, 0.12));
        let ground = meshes.add(Cuboid::new(
            g.cols() as f32 * TILE,
            0.15,
            g.rows() as f32 * TILE,
        ));
        c.spawn((
            Mesh3d(ground),
            MeshMaterial3d(earth),
            Transform::from_xyz(
                g.cols() as f32 * TILE / 2.,
                -0.15,
                g.rows() as f32 * TILE / 2.,
            ),
            ChildOf(root),
        ));
        let wall = meshes.add(rounded_hedge());
        let paving = meshes.add(Cylinder::new(0.58, 0.06).mesh().resolution(12));
        for r in 0..g.rows() {
            for col in 0..g.cols() {
                let x = (col as f32 + 0.5) * TILE;
                let z = (r as f32 + 0.5) * TILE;
                if g.grid[r][col] != 0 {
                    c.spawn((
                        Mesh3d(wall.clone()),
                        MeshMaterial3d(hedge[(r + col) % 3].clone()),
                        // Low opaque core closes gaps beneath the textured foliage.
                        Transform::from_xyz(x, 0.20, z).with_scale(Vec3::new(1., 0.38, 1.)),
                        ChildOf(root),
                    ));
                } else {
                    // Overlapping irregular stones break up the board-game grid.
                    for (i, (dx, dz)) in
                        [(-0.42, -0.43), (0.43, -0.36), (-0.32, 0.43), (0.48, 0.48)]
                            .into_iter()
                            .enumerate()
                    {
                        let variation = ((r * 17 + col * 13 + i * 7) % 11) as f32 / 10.;
                        c.spawn((
                            Mesh3d(paving.clone()),
                            MeshMaterial3d(stones[(r + col + i) % stones.len()].clone()),
                            Transform::from_xyz(x + dx, -0.035, z + dz)
                                .with_rotation(Quat::from_rotation_y(variation * 1.3))
                                .with_scale(Vec3::new(
                                    0.85 + variation * 0.18,
                                    1.,
                                    0.73 + variation * 0.16,
                                )),
                            ChildOf(root),
                        ));
                    }
                }
            }
        }
        let end: [usize; 2] = serde_json::from_value(g.puzzle.data["end"].clone())
            .unwrap_or([g.rows() - 2, g.cols() - 2]);
        let x = (end[1] as f32 + 0.5) * TILE;
        let z = (end[0] as f32 + 0.5) * TILE;
        let brass = material(&mut materials, Color::srgb(0.65, 0.42, 0.15));
        let post = meshes.add(Cylinder::new(0.075, 1.5).mesh().resolution(12));
        for dx in [-0.70, 0.70] {
            c.spawn((
                Mesh3d(post.clone()),
                MeshMaterial3d(brass.clone()),
                Transform::from_xyz(x + dx, 0.75, z),
                ChildOf(root),
            ));
        }
        let arch = meshes.add(Cylinder::new(0.075, 0.285).mesh().resolution(12));
        for i in 0..8 {
            let angle = (i as f32 + 0.5) * std::f32::consts::PI / 8.;
            c.spawn((
                Mesh3d(arch.clone()),
                MeshMaterial3d(brass.clone()),
                Transform::from_xyz(x + angle.cos() * 0.70, 1.5 + angle.sin() * 0.70, z)
                    .with_rotation(Quat::from_rotation_z(angle)),
                ChildOf(root),
            ));
        }
        let beacon = materials.add(StandardMaterial {
            base_color: Color::srgb(1., 0.82, 0.30),
            emissive: LinearRgba::rgb(3., 1.8, 0.3),
            ..default()
        });
        c.spawn((
            Mesh3d(meshes.add(Sphere::new(0.22))),
            MeshMaterial3d(beacon),
            garden_weather::Special,
            Transform::from_xyz(x, 2.4, z),
            ChildOf(root),
        ));
        c.spawn((
            DirectionalLight {
                illuminance: 5000.,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.6, 0.)),
            ChildOf(root),
        ));
        c.insert_resource(AmbientLight {
            color: Color::srgb(1., 0.94, 0.78),
            brightness: 500.,
            ..default()
        });
        c.spawn((
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: 67_f32.to_radians(),
                near: 0.05,
                far: 100.,
                ..default()
            }),
            Camera {
                order: 0,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.055, 0.12, 0.085)),
                ..default()
            },
            Tonemapping::Reinhard,
            Msaa::Off,
            DistanceFog {
                color: Color::srgb(0.055, 0.12, 0.085),
                falloff: FogFalloff::Linear {
                    start: 10.,
                    end: 32.,
                },
                ..default()
            },
            Transform::from_xyz(walk.position.x, 1.45, walk.position.y)
                .with_rotation(Quat::from_rotation_y(walk.yaw)),
            Eyes,
            ChildOf(root),
        ));
        garden_weather::decorate(
            &mut c,
            root,
            &g,
            &assets,
            &mut meshes,
            &mut materials,
            &mut atmosphere,
        );
        let marker = materials.add(StandardMaterial {
            base_color: Color::srgb(0.62, 0.90, 0.53),
            unlit: true,
            ..default()
        });
        c.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.14, 0.012).mesh().resolution(16))),
            MeshMaterial3d(marker),
            Transform::default(),
            Visibility::Hidden,
            Destination,
            garden_weather::Special,
            ChildOf(root),
        ));
        return;
    }
    if p.busy || g.won {
        walk.stop();
    }
    let pad = pad_rect(canvas.height, walk.left_handed);
    let blocked = p.busy || g.won;
    walk.invalid_tap_for = (walk.invalid_tap_for - time.delta_secs()).max(0.);
    // A finger belongs either to the Walk/Stop pad or to looking, never both.
    for touch in touches.iter_just_pressed() {
        let point = (touch.position() - canvas.offset) / canvas.scale;
        if blocked || lost_lantern::hud_blocked(point, &lantern_feed) {
            continue;
        }
        if inside(point, pad) {
            walk.target = None;
            if walk.mode == ControlMode::Hold && walk.move_id.is_none() {
                walk.move_id = Some(touch.id());
            }
        } else if point.x >= 0.
            && point.x < 480.
            && point.y > 182.
            && point.y < canvas.height - 82.
            && walk.look_id.is_none()
        {
            walk.target = None;
            walk.look_id = Some(touch.id());
            walk.look_start = point;
            walk.look_last = point;
            walk.look_max_travel = 0.;
            walk.look_age = 0.;
        }
    }
    walk.stick = Vec2::ZERO;
    if let Some(id) = walk.move_id {
        if let Some(touch) = touches.get_pressed(id) {
            let point = (touch.position() - canvas.offset) / canvas.scale;
            if inside(point, pad) {
                walk.stick.y = -1.;
            } else {
                walk.move_id = None;
            }
        } else {
            walk.move_id = None;
        }
    }
    if let Some(id) = walk.look_id {
        walk.look_age += time.delta_secs();
        if let Some(touch) = touches.get_pressed(id).or_else(|| touches.get_released(id)) {
            let point = (touch.position() - canvas.offset) / canvas.scale;
            walk.look_max_travel = walk
                .look_max_travel
                .max(point.distance_squared(walk.look_start));
            if walk.look_max_travel > 64. {
                let delta = point - walk.look_last;
                walk.yaw -= delta.x * 0.0045;
                walk.pitch = (walk.pitch - delta.y * 0.0035).clamp(-0.65, 0.65);
            }
            walk.look_last = point;
            if touches.get_released(id).is_some() {
                if walk.mode == ControlMode::Tap
                    && garden_controls::is_tap(walk.look_max_travel, walk.look_age)
                {
                    walk.invalid_tap_for = 2.5;
                    if let Some((camera, transform)) = cameras.iter_mut().next() {
                        if let Ok(ray) = camera
                            .viewport_to_world(&GlobalTransform::from(*transform), touch.position())
                        {
                            if ray.direction.y < -0.04 {
                                let distance = -ray.origin.y / ray.direction.y;
                                let ground = ray.get_point(distance);
                                if distance > 0. {
                                    walk.target = garden_controls::destination(
                                        &g.grid,
                                        walk.position.to_array(),
                                        [ground.x, ground.z],
                                    )
                                    .map(Vec2::from_array);
                                    if walk.target.is_some() {
                                        walk.invalid_tap_for = 0.;
                                    }
                                }
                            }
                        }
                    }
                }
                walk.look_id = None;
            }
        } else {
            walk.look_id = None;
        }
    }
    let mut stick = walk.stick;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::Space) {
        stick.y -= 1.;
    }
    if keys.pressed(KeyCode::KeyS) {
        stick.y += 1.;
    }
    if keys.pressed(KeyCode::KeyA) {
        stick.x -= 1.;
    }
    if keys.pressed(KeyCode::KeyD) {
        stick.x += 1.;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        walk.yaw += time.delta_secs() * 1.4;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        walk.yaw -= time.delta_secs() * 1.4;
    }
    if keys.just_pressed(KeyCode::Escape) {
        walk.target = None;
    }
    let dt = time.delta_secs().min(0.05);
    let speed = 1.65;
    let mut delta = (Quat::from_rotation_y(walk.yaw)
        * Vec3::new(stick.x, 0., stick.y).clamp_length_max(1.))
        * dt
        * speed;
    if let Some(target) = walk.target {
        if walk.mode != ControlMode::Tap || stick.length_squared() > 0.01 || blocked {
            walk.target = None;
        } else {
            let offset = target - walk.position;
            if offset.length() < 0.06 {
                walk.target = None;
            } else {
                let step = offset.clamp_length_max(dt * speed);
                delta = Vec3::new(step.x, 0., step.y);
            }
        }
    }
    if blocked {
        delta = Vec3::ZERO;
    }
    let before = walk.position;
    let next = walk_physics::advance(&g.grid, before.to_array(), [delta.x, delta.z]);
    walk.position = Vec2::from_array(next);
    walk.walking = walk.position.distance_squared(before) > 0.000001;
    if delta.length_squared() > 0.000001 && !walk.walking {
        walk.target = None;
    }
    let cell = [
        (next[1] / TILE).floor() as usize,
        (next[0] / TILE).floor() as usize,
    ];
    if g.path.last().copied() != Some(cell) {
        if let Some(game) = &mut p.saved.game {
            game.tap(cell);
        }
        p.message.clear();
        p.dirty = true;
        persist(&p);
    }
    for (mut camera, mut transform) in &mut cameras {
        transform.translation = Vec3::new(next[0], 1.45, next[1]);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, walk.yaw, walk.pitch, 0.);
        if let Ok(window) = windows.single() {
            let [l, t, r, b] = platform::safe_area();
            camera.viewport = Some(bevy::camera::Viewport {
                physical_position: UVec2::new(l as u32, t as u32),
                physical_size: UVec2::new(
                    window
                        .physical_width()
                        .saturating_sub((l + r) as u32)
                        .max(1),
                    window
                        .physical_height()
                        .saturating_sub((t + b) as u32)
                        .max(1),
                ),
                ..default()
            });
        }
    }
}
pub(super) fn hud(
    walk: Res<Walk>,
    mut pads: Query<&mut BackgroundColor, With<WalkPad>>,
    mut markers: Query<(&mut Transform, &mut Visibility), With<Destination>>,
    mut hints: Query<&mut Text, With<ControlHint>>,
) {
    for mut text in &mut hints {
        let hint = if walk.invalid_tap_for > 0. {
            "Tap closer on an open path."
        } else if walk.mode == ControlMode::Hold {
            "Drag the garden to look.\nRelease Walk to stop."
        } else if walk.target.is_some() {
            "Walking to your marker.\nTap Stop to pause."
        } else {
            "Tap nearby path to walk.\nDrag to look. Stop any time."
        };
        if text.0 != hint {
            text.0 = hint.into();
        }
    }
    for mut color in &mut pads {
        color.0 = if walk.walking {
            Color::srgb(0.25, 0.49, 0.26)
        } else {
            Color::srgb(0.055, 0.20, 0.14)
        };
    }
    for (mut transform, mut visibility) in &mut markers {
        *visibility = if walk.target.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(target) = walk.target {
            transform.translation = Vec3::new(target.x, 0.025, target.y);
        }
    }
}
