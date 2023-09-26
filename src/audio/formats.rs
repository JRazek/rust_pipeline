use crate::tags::Tag;

pub mod pcm {
    use super::*;

    #[derive(Debug, Default, Clone)]
    pub struct S16LE;

    #[derive(Debug)]
    pub enum Layout<S> {
        S16LE(Tag<S16LE, Box<[u16]>, S>),
    }

    #[derive(Debug)]
    pub struct Pcm<T> {
        pub layout: Layout<T>,
        pub sample_rate: u32,
    }
}

#[derive(Debug)]
pub enum BitRate {
    Br128,
}

#[derive(Debug)]
pub struct Mulaw {
    pub sample_rate: u32,
}

#[derive(Debug)]
pub struct Mp3 {
    pub bit_rate: BitRate,
}

#[derive(Debug)]
pub enum AudioFormat<T> {
    Mulaw(Mulaw),
    PCM(pcm::Pcm<T>),
}
