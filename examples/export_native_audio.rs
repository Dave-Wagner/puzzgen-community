//! Original procedural soundtracks, generated without third-party recordings.
use std::{f32::consts::TAU, io::Write, path::PathBuf};
fn main() -> std::io::Result<()> {
    let directory = PathBuf::from("apps/android/game/public/audio");
    std::fs::create_dir_all(&directory)?;
    for kind in 0..10 {
        let rate = 22050_u32;
        let bpm = [72., 104., 66., 92., 78., 62., 60., 82., 100., 64.][kind];
        let beat = 60. / bpm;
        let seconds = beat * 16.;
        let count = (seconds * rate as f32) as usize;
        let root = [
            196., 261.626, 146.832, 220., 174.614, 130.813, 110., 164.814, 195.998, 146.832,
        ][kind];
        let scale = [
            0., 4., 7., 12., 7., 4., 2., 7., 0., 4., 9., 12., 9., 7., 4., 2.,
        ];
        let mut data = Vec::with_capacity(count * 2);
        for i in 0..count {
            let t = i as f32 / rate as f32;
            let step = (t / beat) as usize;
            let phase = t % beat;
            let note = root * 2_f32.powf(scale[step % 16] / 12.);
            let envelope = (-phase * (3. + kind as f32 * 0.18)).exp();
            let attack = (phase / 0.018).min(1.);
            let melody =
                (TAU * note * t).sin() + 0.26 * (TAU * note * 2. * t).sin() * (-phase * 7.).exp();
            let pad = (TAU * root * 0.5 * t).sin() * 0.13 + (TAU * root * 0.75 * t).sin() * 0.08;
            let bell = if kind == 8 || kind == 1 {
                (TAU * root * 3. * t).sin() * (-phase * 28.).exp() * 0.10
            } else {
                0.
            };
            let fade = (t / 0.025).min(1.) * ((seconds - t) / 0.04).clamp(0., 1.);
            let sample = ((melody * envelope * attack * 0.22 + pad + bell) * fade * 28000.)
                .clamp(-32767., 32767.) as i16;
            data.extend_from_slice(&sample.to_le_bytes());
        }
        let mut f = std::fs::File::create(directory.join(format!("world-{kind}.wav")))?;
        f.write_all(b"RIFF")?;
        f.write_all(&(36 + data.len() as u32).to_le_bytes())?;
        f.write_all(b"WAVEfmt ")?;
        f.write_all(&16_u32.to_le_bytes())?;
        f.write_all(&1_u16.to_le_bytes())?;
        f.write_all(&1_u16.to_le_bytes())?;
        f.write_all(&rate.to_le_bytes())?;
        f.write_all(&(rate * 2).to_le_bytes())?;
        f.write_all(&2_u16.to_le_bytes())?;
        f.write_all(&16_u16.to_le_bytes())?;
        f.write_all(b"data")?;
        f.write_all(&(data.len() as u32).to_le_bytes())?;
        f.write_all(&data)?;
    }
    println!("Generated 10 original looping soundtracks.");
    Ok(())
}
