pub mod audio_pads;

#[derive(Debug, Clone)]
pub enum Layout {
    S16LE(Option<Box<[i16]>>),
}

#[derive(Debug, Clone)]
pub enum BitRate {
    Br128,
}

#[derive(Debug, Clone, Eq)]
pub struct Mulaw {
    pub data: Option<Box<[u8]>>,
    pub sample_rate: u32,
}

impl PartialEq for Mulaw {
    fn eq(&self, other: &Self) -> bool {
        self.sample_rate == other.sample_rate
    }
}

#[derive(Debug, Clone)]
pub struct Pcm {
    pub layout: Layout,
    pub sample_rate: u32,
}

#[derive(Debug, Clone)]
pub struct Mp3 {
    pub data: Option<Box<[u8]>>,
    pub bit_rate: BitRate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioData {
    Mulaw(Mulaw),
    //    PCM(Layout),
    //    MP3(Mp3),
}
