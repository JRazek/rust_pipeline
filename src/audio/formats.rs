use crate::tags::{Empty, Tag};

pub mod pcm {
    use crate::tags::Empty;

    use super::*;

    #[derive(Debug, Default, Clone)]
    pub struct S16LE;

    #[derive(Debug)]
    pub enum Layout<S> {
        S16LE(Tag<S16LE, Box<[i16]>, S>),
    }

    impl Clone for Layout<Empty> {
        fn clone(&self) -> Self {
            match self {
                Layout::S16LE(tag) => Layout::S16LE(tag.clone()),
            }
        }
    }

    #[derive(Debug)]
    pub struct Pcm<T> {
        pub layout: Layout<T>,
        pub sample_rate: u32,
    }

    impl Clone for Pcm<Empty> {
        fn clone(&self) -> Self {
            Self {
                layout: self.layout.clone(),
                sample_rate: self.sample_rate,
            }
        }
    }
}

#[derive(Debug)]
pub struct Mulaw {
    pub sample_rate: u32,
}

#[derive(Debug)]
pub enum AudioFormat<T> {
    PCM(pcm::Pcm<T>),
}

impl Clone for AudioFormat<Empty> {
    fn clone(&self) -> Self {
        match self {
            AudioFormat::PCM(pcm) => AudioFormat::PCM(pcm.clone()),
        }
    }
}
