//! Mobile-budget weather, surface accumulation, native foliage and a path-aware beacon.
use super::*;
use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};
use std::collections::{HashMap, VecDeque};

pub(super) type Surface = ExtendedMaterial<StandardMaterial, Coating>;
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(super) struct Coating {
    #[uniform(100)]
    pub values: Vec4,
    #[texture(101)]
    #[sampler(102)]
    pub snow: Handle<Image>,
}
impl MaterialExtension for Coating {
    fn fragment_shader() -> ShaderRef {
        "shaders/garden-weather.wgsl".into()
    }
}
#[derive(Component)]
pub(super) struct Special;
#[derive(Component)]
pub(super) struct Sky;
#[derive(Component)]
pub(super) struct Puddle;
#[derive(Component)]
pub(super) struct Flake(pub usize);
#[derive(Component)]
pub(super) struct Foliage(pub f32);
#[derive(Component)]
pub(super) struct Glow(pub f32);
#[derive(Component)]
pub(super) struct PendingPlant(pub(super) Handle<Scene>);
#[derive(Resource, Default)]
pub(super) struct SceneChecks(HashMap<AssetId<Scene>, bool>);
pub(super) fn validate_scenes(
    mut commands: Commands,
    pending: Query<(Entity, &PendingPlant)>,
    scenes: Res<Assets<Scene>>,
    registry: Res<bevy::ecs::reflect::AppTypeRegistry>,
    mut checks: ResMut<SceneChecks>,
) {
    for (entity, pending) in &pending {
        let Some(scene) = scenes.get(&pending.0) else {
            continue;
        };
        let valid =
            *checks
                .0
                .entry(pending.0.id())
                .or_insert_with(|| match scene.clone_with(&registry) {
                    Ok(_) => {
                        info!("Garden scene validated: {:?}", pending.0.path());
                        true
                    }
                    Err(error) => {
                        error!("Garden scene rejected before spawning: {error}");
                        false
                    }
                });
        let mut e = commands.entity(entity);
        e.remove::<PendingPlant>();
        if valid {
            e.insert(SceneRoot(pending.0.clone()));
        }
    }
}
#[derive(Resource, Default)]
pub(super) struct Atmosphere {
    coated: HashMap<AssetId<StandardMaterial>, Handle<Surface>>,
    pub snow: f32,
    pub wet: f32,
    rain_mesh: Handle<Mesh>,
    snow_mesh: Handle<Mesh>,
    rain_material: Handle<StandardMaterial>,
    snow_material: Handle<StandardMaterial>,
    sky_material: Handle<StandardMaterial>,
    puddle_material: Handle<StandardMaterial>,
    day: Handle<Image>,
    night: Handle<Image>,
    distances: Vec<Vec<usize>>,
    longest: f32,
    proximity: f32,
    exit: Vec3,
    age: f32,
    report_at: f32,
}
pub(super) fn decorate(
    c: &mut Commands,
    root: Entity,
    g: &Game,
    assets: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    a: &mut Atmosphere,
) {
    a.age = 0.;
    a.coated.clear();
    let player = g.path.last().copied().unwrap_or([1, 1]);
    let mut nearby_ornaments = 0;
    let end: [usize; 2] = serde_json::from_value(g.puzzle.data["end"].clone())
        .unwrap_or([g.rows() - 2, g.cols() - 2]);
    a.exit = Vec3::new((end[1] as f32 + 0.5) * 2., 2.05, (end[0] as f32 + 0.5) * 2.);
    a.distances = vec![vec![usize::MAX; g.cols()]; g.rows()];
    a.distances[end[0]][end[1]] = 0;
    let mut queue = VecDeque::from([end]);
    while let Some([r, col]) = queue.pop_front() {
        for (dr, dc) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let nr = r.wrapping_add_signed(dr);
            let nc = col.wrapping_add_signed(dc);
            if nr < g.rows()
                && nc < g.cols()
                && g.grid[nr][nc] == 0
                && a.distances[nr][nc] == usize::MAX
            {
                a.distances[nr][nc] = a.distances[r][col] + 1;
                queue.push_back([nr, nc]);
            }
        }
    }
    a.longest = a
        .distances
        .iter()
        .flatten()
        .copied()
        .filter(|d| *d != usize::MAX)
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    a.day = assets.load("art/garden3d/skybox-day.png");
    a.night = assets.load("art/garden3d/skybox-night.png");
    a.sky_material = materials.add(StandardMaterial {
        base_color_texture: Some(a.day.clone()),
        unlit: true,
        cull_mode: None,
        fog_enabled: false,
        ..default()
    });
    c.spawn((
        Sky,
        Special,
        Mesh3d(meshes.add(Sphere::new(70.).mesh().uv(48, 24))),
        MeshMaterial3d(a.sky_material.clone()),
        // Bevy's UV sphere has Z-axis poles; equirectangular skies need Y-up.
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ChildOf(root),
    ));
    a.puddle_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.22, 0.37, 0.40, 0.72),
        perceptual_roughness: 0.12,
        metallic: 0.35,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let pool = meshes.add(Cylinder::new(0.6, 0.006));
    for r in 0..g.rows() {
        for col in 0..g.cols() {
            let x = (col as f32 + 0.5) * 2.;
            let z = (r as f32 + 0.5) * 2.;
            if g.grid[r][col] == 0 && (r * 11 + col * 7) % 5 == 0 {
                c.spawn((
                    Puddle,
                    Special,
                    Mesh3d(pool.clone()),
                    MeshMaterial3d(a.puddle_material.clone()),
                    Transform::from_xyz(x + 0.15, 0.007, z - 0.15).with_scale(Vec3::ZERO),
                    Visibility::Hidden,
                    ChildOf(root),
                ));
            }
            // Dress only wall faces beside a path, sharing the imported scene handles.
            // This keeps the maze collision intact without filling unseen solid cells.
            if g.grid[r][col] != 0 {
                let mut exposed = false;
                for (dr, dc) in [(-1_isize, 0_isize), (1, 0), (0, -1), (0, 1)] {
                    let nr = r as isize + dr;
                    let nc = col as isize + dc;
                    if nr < 0
                        || nc < 0
                        || g.grid.get(nr as usize).and_then(|row| row.get(nc as usize)) != Some(&0)
                    {
                        continue;
                    }
                    exposed = true;
                    for (i, tangent) in [-0.45, 0., 0.45].into_iter().enumerate() {
                        let model = if (r * 7 + col * 3 + i) % 5 == 0 {
                            "Bush_Flowers"
                        } else {
                            "Bush"
                        };
                        c.spawn((
                            PendingPlant(assets.load(format!("art/garden3d/{model}.gltf#Scene0"))),
                            Transform::from_xyz(
                                x + dc as f32 * 0.25 + dr as f32 * tangent,
                                0.,
                                z + dr as f32 * 0.25 + dc as f32 * tangent,
                            )
                            .with_scale(Vec3::splat(1.23 + ((r + col + i) % 3) as f32 * 0.035))
                            .with_rotation(Quat::from_rotation_y(
                                (r * 13 + col * 7 + i * 3) as f32 * 1.7,
                            )),
                            Foliage((r + col + i) as f32),
                            ChildOf(root),
                        ));
                    }
                }
                if exposed && (r + col) % 3 == 0 {
                    c.spawn((
                        PendingPlant(assets.load("art/garden3d/Flower_4_Clump.gltf#Scene0")),
                        Transform::from_xyz(x - 0.72, 0., z - 0.72).with_scale(Vec3::splat(0.35)),
                        Foliage(r as f32),
                        ChildOf(root),
                    ));
                }
            }
            // Small ornaments sit inside wall cells, so none blocks the playable path.
            let nearby = r.abs_diff(player[0]) + col.abs_diff(player[1]) <= 2;
            if g.grid[r][col] != 0
                && ((nearby && nearby_ornaments < 3) || (r * 19 + col * 11) % 17 == 0)
            {
                let beside_path = [
                    (r.wrapping_sub(1), col),
                    (r + 1, col),
                    (r, col.wrapping_sub(1)),
                    (r, col + 1),
                ]
                .into_iter()
                .find(|&(nr, nc)| g.grid.get(nr).and_then(|row| row.get(nc)) == Some(&0));
                if let Some((nr, nc)) = beside_path {
                    let model = match if nearby {
                        nearby_ornaments
                    } else {
                        (r + col) % 3
                    } {
                        0 => "Birdhouse",
                        1 => "Lantern",
                        _ => "Mushrooms",
                    };
                    c.spawn((
                        PendingPlant(
                            assets.load(format!("art/garden3d/ornaments/{model}.glb#Scene0")),
                        ),
                        Transform::from_xyz(
                            x + (nc as f32 - col as f32) * 0.90,
                            0.,
                            z + (nr as f32 - r as f32) * 0.90,
                        )
                        .with_rotation(Quat::from_rotation_y(
                            (nc as f32 - col as f32).atan2(nr as f32 - r as f32),
                        )),
                        ChildOf(root),
                    ));
                    if nearby {
                        nearby_ornaments += 1;
                    }
                }
            }
        }
    }
    // A ring of trees outside the collision boundary frames the sky without hiding paths.
    let w = g.cols() as f32 * 2.;
    let h = g.rows() as f32 * 2.;
    for i in 0..16 {
        let side = i / 4;
        let t = (i % 4) as f32 / 3.;
        let (x, z) = match side {
            0 => (-3., t * h),
            1 => (w + 3., t * h),
            2 => (t * w, -3.),
            _ => (t * w, h + 3.),
        };
        let model = if i % 3 == 0 {
            "MapleTree_1"
        } else {
            "BirchTree_1"
        };
        c.spawn((
            PendingPlant(assets.load(format!("art/garden3d/{model}.gltf#Scene0"))),
            Transform::from_xyz(x, 0., z)
                .with_scale(Vec3::splat(4. + (i % 3) as f32))
                .with_rotation(Quat::from_rotation_y(i as f32 * 1.7)),
            ChildOf(root),
        ));
    }
    a.rain_mesh = meshes.add(Cuboid::new(0.012, 0.24, 0.012));
    a.snow_mesh = meshes.add(
        Sphere::new(0.032)
            .mesh()
            .ico(1)
            .expect("valid low-poly flake"),
    );
    a.rain_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.63, 0.82, 0.9, 0.5),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    a.snow_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.94, 0.97, 1.),
        unlit: true,
        ..default()
    });
    for i in 0..180 {
        c.spawn((
            Flake(i),
            Special,
            Mesh3d(a.rain_mesh.clone()),
            MeshMaterial3d(a.rain_material.clone()),
            Transform::default(),
            Visibility::Hidden,
            ChildOf(root),
        ));
    }
    let glow = materials.add(StandardMaterial {
        base_color: Color::srgb(1., 0.82, 0.30),
        emissive: LinearRgba::rgb(4., 2.5, 0.35),
        unlit: true,
        ..default()
    });
    let mote = meshes.add(Sphere::new(0.035).mesh().ico(1).expect("valid mote"));
    for i in 0..18 {
        c.spawn((
            Glow(i as f32),
            Special,
            Mesh3d(mote.clone()),
            MeshMaterial3d(glow.clone()),
            Transform::from_translation(a.exit),
            ChildOf(root),
        ));
    }
    c.spawn((
        PointLight {
            color: Color::srgb(1., 0.73, 0.24),
            intensity: 18000.,
            range: 5.,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(a.exit),
        ChildOf(root),
    ));
}
// Convert imported materials once, keeping the original alpha cutouts and texture handles.
#[allow(clippy::type_complexity)]
pub(super) fn coat(
    mut c: Commands,
    meshes: Query<(Entity, &MeshMaterial3d<StandardMaterial>), Without<Special>>,
    materials: Res<Assets<StandardMaterial>>,
    mut surfaces: ResMut<Assets<Surface>>,
    assets: Res<AssetServer>,
    mut a: ResMut<Atmosphere>,
) {
    for (e, h) in &meshes {
        let Some(base) = materials.get(&h.0) else {
            continue;
        };
        let handle = if let Some(handle) = a.coated.get(&h.0.id()) {
            handle.clone()
        } else {
            let handle = surfaces.add(Surface {
                base: base.clone(),
                extension: Coating {
                    values: Vec4::ZERO,
                    snow: assets.load("art/garden3d/snow.jpg"),
                },
            });
            a.coated.insert(h.0.id(), handle.clone());
            handle
        };
        c.entity(e)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle));
    }
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn animate(
    time: Res<Time>,
    p: Res<Pocket>,
    walk: Res<garden_walk::Walk>,
    mut a: ResMut<Atmosphere>,
    mut surfaces: ResMut<Assets<Surface>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ambient: ResMut<AmbientLight>,
    mut sky: Query<
        &mut Transform,
        (
            With<Sky>,
            Without<Flake>,
            Without<Puddle>,
            Without<Glow>,
            Without<Foliage>,
        ),
    >,
    mut drops: Query<
        (
            &Flake,
            &mut Transform,
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Visibility,
        ),
        (
            Without<Sky>,
            Without<Puddle>,
            Without<Glow>,
            Without<Foliage>,
        ),
    >,
    mut pools: Query<
        (&mut Transform, &mut Visibility),
        (
            With<Puddle>,
            Without<Sky>,
            Without<Flake>,
            Without<Glow>,
            Without<Foliage>,
        ),
    >,
    mut glows: Query<
        (&Glow, &mut Transform),
        (
            Without<Sky>,
            Without<Flake>,
            Without<Puddle>,
            Without<Foliage>,
        ),
    >,
    mut foliage: Query<
        (&Foliage, &mut Transform),
        (Without<Sky>, Without<Flake>, Without<Puddle>, Without<Glow>),
    >,
    mut cameras: Query<&mut DistanceFog, With<garden_walk::Eyes>>,
    mut lights: Query<&mut DirectionalLight>,
    windows: Query<&Window>,
    canvas: Res<Canvas>,
) {
    if !garden_walk::active(&p) {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    a.age += dt;
    let conditions = weather::conditions(&p);
    let night = conditions.night;
    // Fast enough to review, slow enough that changing moods never pops the whole garden.
    let snow_target = conditions.snow;
    a.snow += (snow_target - a.snow) * (dt / 18.).min(1.);
    a.wet += (conditions.rain.max(if a.snow > 0.05 && snow_target == 0. {
        0.5
    } else {
        0.
    }) - a.wet)
        * (dt / 12.).min(1.);
    for (_, s) in surfaces.iter_mut() {
        s.extension.values = Vec4::new(a.snow, a.wet, a.age, conditions.wind);
    }
    let fog = if night {
        Color::srgb(0.10, 0.16, 0.25)
    } else {
        Color::srgb(
            0.63 - conditions.cloud * 0.25,
            0.76 - conditions.cloud * 0.28,
            0.76 - conditions.cloud * 0.20,
        )
    };
    for mut camera in &mut cameras {
        camera.color = fog;
        camera.falloff = FogFalloff::Linear {
            start: 12.,
            end: if conditions.cloud > 0.7 { 38. } else { 58. },
        };
    }
    ambient.color = if night {
        Color::srgb(0.60, 0.72, 1.)
    } else {
        Color::srgb(1., 0.94, 0.84)
    };
    ambient.brightness = if night {
        180.
    } else {
        500. - conditions.cloud * 150.
    };
    for mut light in &mut lights {
        light.illuminance = if night {
            650.
        } else {
            5000. - conditions.cloud * 3400.
        };
    }
    if let Some(s) = materials.get_mut(&a.sky_material) {
        s.base_color_texture = Some(if night {
            a.night.clone()
        } else {
            a.day.clone()
        });
        s.base_color = if night {
            Color::srgb(0.65, 0.72, 0.95)
        } else {
            Color::srgb(
                1. - conditions.cloud * 0.55,
                1. - conditions.cloud * 0.5,
                1. - conditions.cloud * 0.38,
            )
        };
    }
    let pos = walk.position;
    for mut t in &mut sky {
        t.translation = Vec3::new(pos.x, 0., pos.y);
    }
    for (mut t, mut visibility) in &mut pools {
        let amount = (a.wet - a.snow * 0.8).max(0.);
        // Zero scale does not exclude a mesh from render extraction/pipeline creation.
        *visibility = if amount > 0.001 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        t.scale = Vec3::new(amount, 1., amount * 0.65);
    }
    let snowing = conditions.falling_snow;
    let count = if p.saved.reduced_motion {
        0
    } else if snowing {
        100
    } else {
        (conditions.rain * 180.) as usize
    };
    for (f, mut t, mut mesh, mut mat, mut visibility) in &mut drops {
        *visibility = if f.0 < count {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if f.0 >= count {
            continue;
        }
        mesh.0 = if snowing {
            a.snow_mesh.clone()
        } else {
            a.rain_mesh.clone()
        };
        mat.0 = if snowing {
            a.snow_material.clone()
        } else {
            a.rain_material.clone()
        };
        let seed = f.0 as f32;
        let speed = if snowing { 0.7 } else { 8. };
        let x = (seed * 7.31 + a.age * conditions.wind * 0.8).rem_euclid(18.) - 9. + pos.x;
        let z = (seed * 3.71 + a.age * conditions.wind * 0.3).rem_euclid(18.) - 9. + pos.y;
        let floor = p
            .saved
            .game
            .as_ref()
            .and_then(|g| {
                g.grid
                    .get((z / 2.).floor().max(0.) as usize)
                    .and_then(|row| row.get((x / 2.).floor().max(0.) as usize))
            })
            .map_or(0., |v| if *v == 0 { 0. } else { 1.3 });
        t.translation = Vec3::new(x, floor + (seed * 0.713 - a.age * speed).rem_euclid(6.), z);
        t.rotation = Quat::from_rotation_z(if snowing { 0. } else { conditions.wind * 0.18 });
    }
    for (f, mut t) in &mut foliage {
        t.rotation = Quat::from_rotation_z(if p.saved.reduced_motion {
            0.
        } else {
            (a.age * 1.1 + f.0).sin() * conditions.wind * 0.025
        });
    }
    let cell = [(pos.y / 2.) as usize, (pos.x / 2.) as usize];
    let distance = a
        .distances
        .get(cell[0])
        .and_then(|r| r.get(cell[1]))
        .copied()
        .unwrap_or(usize::MAX);
    let target = if distance == usize::MAX {
        0.
    } else {
        1. - distance as f32 / a.longest
    };
    a.proximity += (target - a.proximity) * (dt / 2.).min(1.);
    let strength = match p.saved.game.as_ref().map(|g| g.puzzle.difficulty.as_str()) {
        Some("hard") => 0.15,
        Some("medium") => 0.45,
        _ => 1.,
    };
    for (g, mut t) in &mut glows {
        let phase = g.0 * 2.399;
        let age = if p.saved.reduced_motion { 0. } else { a.age };
        let radius = 0.35 + 0.6 * a.proximity * strength;
        t.translation = a.exit
            + Vec3::new(
                (phase + age * 0.35).cos() * radius,
                (phase * 1.7 + age).sin() * 0.35 + (g.0 / 18.) * 0.7,
                (phase + age * 0.35).sin() * radius,
            );
        t.scale = Vec3::splat(0.65 + a.proximity * strength + (age * 1.5 + phase).sin() * 0.15);
    }
    // Debug-only observability for emulator review; never includes credentials or a city.
    // This avoids hidden test buttons and lets automation use the real safe-area transform.
    if platform::trace_input() && time.elapsed_secs() - a.report_at > 2. {
        a.report_at = time.elapsed_secs();
        if let Ok(window) = windows.single() {
            info!(
                "WEATHER_REVIEW {}",
                json!({
                    "mode":p.saved.weather.mode,"snow":a.snow,"wet":a.wet,
                    "particles":count,"gentle_motion":p.saved.reduced_motion,
                    "physical":[window.physical_width(),window.physical_height()],
                    "logical":[window.width(),window.height()],
                    "window_scale":window.scale_factor(),
                    "canvas_scale":canvas.scale,"canvas_offset":canvas.offset.to_array(),
                    "safe":platform::safe_area(),"audio_allowed":audio_policy::allowed()
                })
            );
        }
    }
}
