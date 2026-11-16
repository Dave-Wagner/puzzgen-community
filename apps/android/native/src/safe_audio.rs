use super::audio_policy;
use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    audio::{AudioSource, Decodable, Source},
    prelude::*,
};
#[derive(Asset, TypePath, Clone)]
pub(super) struct SafeAudio(AudioSource);
pub(super) struct SafeDecoder(<AudioSource as Decodable>::Decoder);
impl Iterator for SafeDecoder {
    type Item = i16;
    fn next(&mut self) -> Option<i16> {
        self.0.next().map(audio_policy::sample)
    }
}
impl Source for SafeDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        self.0.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.0.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.0.sample_rate()
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.0.total_duration()
    }
}
impl Decodable for SafeAudio {
    type DecoderItem = i16;
    type Decoder = SafeDecoder;
    fn decoder(&self) -> Self::Decoder {
        SafeDecoder(self.0.decoder())
    }
}
#[derive(Default)]
pub(super) struct SafeLoader;
impl AssetLoader for SafeLoader {
    type Asset = SafeAudio;
    type Settings = ();
    type Error = std::io::Error;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> Result<SafeAudio, std::io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(SafeAudio(AudioSource {
            bytes: bytes.into(),
        }))
    }
    fn extensions(&self) -> &[&str] {
        &["wav", "ogg", "mp3"]
    }
}
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_davewagner_puzzgen_MainActivity_nativeAudioAllowed(
    _env: jni::JNIEnv<'_>,
    _class: jni::objects::JClass<'_>,
    allowed: jni::sys::jboolean,
) {
    audio_policy::set_active(allowed != 0);
}
