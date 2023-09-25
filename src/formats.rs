use super::audio::AudioData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaggedData {
    Audio(AudioData),
}
