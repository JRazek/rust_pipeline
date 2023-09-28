use crate::tags::Full;

use super::tags::Empty;

use super::audio::AudioFormat;

#[derive(Debug)]
pub enum MediaFormat<S = Empty> {
    Audio(AudioFormat<S>),
}

pub type MediaData = MediaFormat<Full>;

impl Clone for MediaFormat<Empty> {
    fn clone(&self) -> Self {
        match self {
            MediaFormat::Audio(audio) => MediaFormat::Audio(audio.clone()),
        }
    }
}
