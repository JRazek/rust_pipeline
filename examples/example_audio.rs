use pipeline::errors::LinkError;
use pipeline::pad::{FormatNegotiator, FormatProvider, LinkElement, SinkPad, StreamPad};

use tokio::sync::mpsc as tokio_mpsc;

use pipeline::channel_traits::mpsc as pipeline_mpsc;

use pipeline::tags::{Empty, Full, Tag};

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

pub mod pcm {
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

pub struct Sender(pub tokio_mpsc::Sender<MediaData>);

impl Drop for Sender {
    fn drop(&mut self) {
        println!("Dropping sender");
    }
}

pub struct Receiver(pub tokio_mpsc::Receiver<MediaData>);

impl Drop for Receiver {
    fn drop(&mut self) {
        println!("Dropping receiver");
    }
}

#[async_trait::async_trait]
impl pipeline_mpsc::Sender<MediaData> for Sender {
    async fn send(&self, data: MediaData) -> Result<(), pipeline_mpsc::SendError<MediaData>> {
        let tx = &self.0;

        tx.send(data)
            .await
            .map_err(|tokio_mpsc::error::SendError(data)| pipeline_mpsc::SendError(data))
    }
}

#[async_trait::async_trait]
impl pipeline_mpsc::Receiver<MediaData> for Receiver {
    async fn recv(&mut self) -> Option<MediaData> {
        let rx = &mut self.0;

        rx.recv().await
    }
}

fn frequencies_to_s16_pcm(freq: &Vec<u32>) -> Vec<MediaFormat> {
    freq.iter()
        .map(|&sample_rate| {
            MediaFormat::Audio(AudioFormat::PCM(pcm::Pcm {
                layout: pcm::Layout::S16LE(Tag::default()),
                sample_rate,
            }))
        })
        .collect()
}

struct AudioProducerStreamPad {
    rx: Receiver,
}

impl FormatProvider<MediaFormat> for AudioProducerStreamPad {
    fn formats(&self) -> Vec<MediaFormat> {
        frequencies_to_s16_pcm(&vec![16000])
    }
}

impl StreamPad<MediaData, MediaFormat> for AudioProducerStreamPad {
    type Receiver = Receiver;

    fn get_rx(self, format: &MediaFormat) -> Result<Self::Receiver, pipeline::errors::LinkError> {
        match &format {
            MediaFormat::Audio(AudioFormat::PCM(pcm::Pcm {
                layout: pcm::Layout::S16LE(_),
                sample_rate: 16000,
            })) => {
                let rx = self.rx;

                Ok(rx)
            }
            _ => Err(pipeline::errors::LinkError::InitialFormatMismatch),
        }
    }
}

fn audio_producer(rt: &tokio::runtime::Runtime) -> AudioProducerStreamPad {
    const SAMPLE_RATE: u32 = 16000;

    let sample_duration =
        std::time::Duration::from_nanos((1. / SAMPLE_RATE as f64 * 1000_000.) as u64);

    let mut time = std::time::Duration::ZERO;

    let (tx, rx) = tokio_mpsc::channel(1024);

    let task = async move {
        loop {
            let mut buffer = vec![0i16; 16];

            for i in 0..buffer.len() {
                buffer[i] = signal(time);

                time += sample_duration;
            }

            let data = buffer.into_boxed_slice();

            let buffer = MediaData::Audio(AudioFormat::PCM(pcm::Pcm {
                layout: pcm::Layout::S16LE(Tag::new(data)),
                sample_rate: SAMPLE_RATE,
            }));

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            tx.send(buffer).await.unwrap();
        }
    };

    rt.spawn(task);

    AudioProducerStreamPad { rx: Receiver(rx) }
}

struct PassthroughSinkPad {
    tx: Sender,
}

impl SinkPad<MediaData, MediaFormat> for PassthroughSinkPad {
    type Sender = Sender;

    fn get_tx(self, _: &MediaFormat) -> Result<Self::Sender, pipeline::errors::LinkError> {
        Ok(self.tx)
    }
}

struct PassthroughStreamPad {
    supported_formats: Vec<MediaFormat>,
    rx: Receiver,
}

impl StreamPad<MediaData, MediaFormat> for PassthroughStreamPad {
    type Receiver = Receiver;

    fn get_rx(self, _: &MediaFormat) -> Result<Self::Receiver, pipeline::errors::LinkError> {
        Ok(self.rx)
    }
}

impl FormatProvider<MediaFormat> for PassthroughStreamPad {
    fn formats(&self) -> Vec<MediaFormat> {
        self.supported_formats.clone()
    }
}

struct PassthroughLink<'a> {
    rt: &'a tokio::runtime::Runtime,
}

impl LinkElement<MediaData, MediaFormat> for PassthroughLink<'_> {
    type SinkPad = PassthroughSinkPad;
    type StreamPad = PassthroughStreamPad;

    fn get_pads(
        self,
        rx_format: &MediaFormat,
    ) -> Result<(Self::SinkPad, Self::StreamPad), LinkError> {
        let (sink_pad, stream_pad) = spawn_passthrough_task(self.rt);

        let sink_pad = PassthroughSinkPad { tx: sink_pad };
        let stream_pad = PassthroughStreamPad {
            supported_formats: vec![rx_format.clone()],
            rx: stream_pad,
        };

        Ok((sink_pad, stream_pad))
    }
}

impl FormatNegotiator<MediaFormat> for PassthroughLink<'_> {
    fn matches(&self, _: &MediaFormat) -> bool {
        true
    }
}

fn spawn_passthrough_task(rt: &tokio::runtime::Runtime) -> (Sender, Receiver) {
    let (in_tx, in_rx) = tokio_mpsc::channel(1024);
    let (out_tx, out_rx) = tokio_mpsc::channel(1024);

    rt.spawn(passthrough_task(Receiver(in_rx), Sender(out_tx)));

    (Sender(in_tx), Receiver(out_rx))
}

async fn passthrough_task(mut rx: Receiver, tx: Sender) {
    let rx = &mut rx.0;
    let tx = &tx.0;

    while let Some(data) = rx.recv().await {
        tx.send(data).await.unwrap();
    }

    eprintln!("passthrough task finished");
}

fn signal(time: std::time::Duration) -> i16 {
    let time = time.as_nanos() as f64;

    let converted = (f64::sin(time * 0.001) * i16::MAX as f64) as i16;

    converted
}

struct ConsumerPad {
    tx: Sender,
}

fn consumer_task(rt: &tokio::runtime::Runtime) -> ConsumerPad {
    let (tx, mut rx) = tokio_mpsc::channel(1024);

    let task = async move {
        while let Some(data) = rx.recv().await {
            match &data {
                MediaData::Audio(AudioFormat::PCM(pcm::Pcm {
                    layout: pcm::Layout::S16LE(data),
                    sample_rate: 16000,
                })) => {
                    println!("Got audio data: {:?}", data);
                }
                _ => {
                    panic!("Got unexpected data");
                }
            }
        }

        eprintln!("consumer task finished");
    };

    rt.spawn(task);

    ConsumerPad { tx: Sender(tx) }
}

impl FormatNegotiator<MediaFormat> for ConsumerPad {
    fn matches(&self, format: &MediaFormat) -> bool {
        match format {
            MediaFormat::Audio(AudioFormat::PCM(pcm::Pcm {
                layout: pcm::Layout::S16LE(_),
                sample_rate: 16000,
            })) => true,
            _ => false,
        }
    }
}

impl SinkPad<MediaData, MediaFormat> for ConsumerPad {
    type Sender = Sender;

    fn get_tx(self, _: &MediaFormat) -> Result<Self::Sender, pipeline::errors::LinkError> {
        Ok(self.tx)
    }
}

fn main() {
    use pipeline::pipeline_builder::PipelineBuilder;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let producer_stream_pad = audio_producer(&rt);

    let passthrough_link = PassthroughLink { rt: &rt };

    let consumer_pad = consumer_task(&rt);

    PipelineBuilder::with_stream(producer_stream_pad)
        .set_link(passthrough_link, &rt)
        .unwrap()
        .build_with_sink(consumer_pad, &rt)
        .unwrap();

    rt.block_on(async {
        tokio::signal::ctrl_c().await.unwrap();
    });
}
