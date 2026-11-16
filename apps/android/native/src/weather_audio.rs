use super::*;
#[derive(Component)]
pub(super) struct WeatherBed {
    kind: usize,
    gain: f32,
}
#[derive(Component)]
pub(super) struct ThunderRoll;
#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    mut commands: Commands,
    time: Res<Time>,
    p: Res<Pocket>,
    assets: Res<AssetServer>,
    mut beds: Query<(Entity, &mut WeatherBed, Option<&mut AudioSink>)>,
    mut thunder: Local<f32>,
    rolls: Query<Entity, With<ThunderRoll>>,
) {
    let weather = weather::conditions(&p);
    let active = garden_walk::active(&p) && audio_policy::allowed();
    let targets = if active {
        [weather.rain * 0.32, weather.wind * 0.4]
    } else {
        [0., 0.]
    };
    let mut present = [false; 2];
    for (entity, mut bed, sink) in &mut beds {
        present[bed.kind] = true;
        let target = targets[bed.kind];
        if let Some(mut sink) = sink {
            if active {
                sink.play();
                bed.gain += (target - bed.gain) * (time.delta_secs().min(0.05) / 1.5);
            } else {
                sink.pause();
                bed.gain = 0.;
            }
            sink.set_volume(bevy::audio::Volume::Linear(bed.gain));
            if target == 0. && bed.gain < 0.001 {
                commands.entity(entity).despawn();
            }
        }
    }
    for kind in 0..2 {
        if !present[kind] && targets[kind] > 0. {
            commands.spawn((
                AudioPlayer::<SafeAudio>(assets.load(if kind == 0 {
                    "audio/garden-rain.wav"
                } else {
                    "audio/garden-wind.wav"
                })),
                PlaybackSettings {
                    paused: true,
                    ..PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::Linear(0.))
                },
                WeatherBed { kind, gain: 0. },
            ));
        }
    }
    if !active || !weather.storm || !p.saved.weather.thunder {
        for roll in &rolls {
            commands.entity(roll).despawn();
        }
        *thunder = 0.;
        return;
    }
    *thunder += time.delta_secs().min(0.05);
    if *thunder > 37. {
        *thunder = 0.;
        commands.spawn((
            ThunderRoll,
            AudioPlayer::<SafeAudio>(assets.load("audio/garden-thunder.wav")),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.25)),
        ));
    }
}
