pub mod audio_pads;

#[derive(Debug, Clone)]
pub enum Layout {
    S16LE(Option<Box<[i16]>>),
}

#[derive(Debug, Clone)]
pub enum BitRate {
    Br128,
    Br192,
    Br256,
    Br320,
}

#[derive(Debug, Clone)]
pub struct Mulaw {
    pub data: Option<Box<[u8]>>,
    pub sample_rate: u32,
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

#[derive(Debug, Clone)]
pub enum AudioData {
    Mulaw(Mulaw),
    PCM(Layout),
    MP3(Mp3),
}
