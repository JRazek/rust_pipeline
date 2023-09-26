use super::tags::Empty;

use super::audio::AudioFormat;

#[derive(Debug, PartialEq, Eq)]
pub enum Format<S = Empty> {
    Audio(AudioFormat<S>),
}
