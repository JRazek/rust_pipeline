use crate::tags::Tag;

pub mod pcm {
    use super::*;

    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    pub struct S16LE;

    #[derive(Debug, PartialEq, Eq)]
    pub enum Layout<S> {
        S16LE(Tag<S16LE, Box<[i16]>, S>),
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct Pcm<T> {
        pub layout: Layout<T>,
        pub sample_rate: u32,
    }
}

#[derive(Debug)]
pub enum BitRate {
    Br128,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Mulaw {
    pub sample_rate: u32,
}

#[derive(Debug)]
pub struct Mp3 {
    pub bit_rate: BitRate,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AudioFormat<T> {
    Mulaw(Mulaw),
    PCM(pcm::Pcm<T>),
}
