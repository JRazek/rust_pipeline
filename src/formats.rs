use super::audio::AudioFormat;
use super::tags::Empty;

#[derive(Debug)]
pub enum Format<S = Empty> {
    Audio(AudioFormat<S>),
}
