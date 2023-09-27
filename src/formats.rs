use crate::tags::Full;

use super::tags::Empty;

use super::audio::AudioFormat;

#[derive(Debug, PartialEq, Eq)]
pub enum MediaFormat<S = Empty> {
    Audio(AudioFormat<S>),
}

pub type MediaData = MediaFormat<Full>;
