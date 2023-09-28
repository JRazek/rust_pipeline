use pipeline::audio::pcm::{Layout, Pcm};
use pipeline::audio::AudioFormat;
use pipeline::pad_element::{SinkPad, StreamPad};
use pipeline::pads::{self, FormatNegotiator, FormatProvider, MediaData, MediaFormat};
use pipeline::tags::Tag;

use tokio::sync::mpsc as tokio_mpsc;

use pipeline::channel_traits::mpsc as pipeline_mpsc;

pub struct Sender(pub tokio_mpsc::Sender<pads::MediaData>);

impl Drop for Sender {
    fn drop(&mut self) {
        println!("Dropping sender");
    }
}

pub struct Receiver(pub tokio_mpsc::Receiver<pads::MediaData>);

impl Drop for Receiver {
    fn drop(&mut self) {
        println!("Dropping receiver");
    }
}

#[async_trait::async_trait]
impl pipeline_mpsc::Sender<pads::MediaData> for Sender {
    async fn send(
        &self,
        data: pads::MediaData,
    ) -> Result<(), pipeline_mpsc::SendError<pads::MediaData>> {
        let tx = &self.0;

        tx.send(data)
            .await
            .map_err(|tokio_mpsc::error::SendError(data)| pipeline_mpsc::SendError(data))
    }
}

#[async_trait::async_trait]
impl pipeline_mpsc::Receiver<pads::MediaData> for Receiver {
    async fn recv(&mut self) -> Option<pads::MediaData> {
        let rx = &mut self.0;

        rx.recv().await
    }
}

fn frequencies_to_s16_pcm(freq: &Vec<u32>) -> Vec<pads::MediaFormat> {
    freq.iter()
        .map(|&sample_rate| {
            pads::MediaFormat::Audio(pipeline::audio::AudioFormat::PCM(
                pipeline::audio::pcm::Pcm {
                    layout: pipeline::audio::pcm::Layout::S16LE(Tag::default()),
                    sample_rate,
                },
            ))
        })
        .collect()
}

struct PassthroughSinkPad {
    tx: Sender,
}

impl FormatNegotiator<MediaFormat> for PassthroughSinkPad {
    fn matches(&self, _: &pads::MediaFormat) -> bool {
        true
    }
}

impl SinkPad<MediaData, MediaFormat> for PassthroughSinkPad {
    type Sender = Sender;

    fn get_tx(self, _: &pads::MediaFormat) -> Result<Self::Sender, pipeline::errors::LinkError> {
        Ok(self.tx)
    }
}

struct PassthroughStreamPad {
    supported_frequencies: Vec<u32>,
    rx: Receiver,
}

impl StreamPad<MediaData, MediaFormat> for PassthroughStreamPad {
    type Receiver = Receiver;

    fn get_rx(self, _: &pads::MediaFormat) -> Result<Self::Receiver, pipeline::errors::LinkError> {
        Ok(self.rx)
    }
}

impl FormatProvider<MediaFormat> for PassthroughStreamPad {
    fn formats(&self) -> Vec<pads::MediaFormat> {
        frequencies_to_s16_pcm(&self.supported_frequencies)
    }
}

fn spawn_passthrough_task(
    rt: &tokio::runtime::Runtime,
) -> (PassthroughSinkPad, PassthroughStreamPad) {
    let (in_tx, in_rx) = tokio_mpsc::channel(1024);
    let (out_tx, out_rx) = tokio_mpsc::channel(1024);

    rt.spawn(passthrough_task(Receiver(in_rx), Sender(out_tx)));

    (
        PassthroughSinkPad { tx: Sender(in_tx) },
        PassthroughStreamPad {
            supported_frequencies: vec![16000],
            rx: Receiver(out_rx),
        },
    )
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

struct AudioProducerStreamPad {
    rx: Receiver,
}

impl FormatProvider<MediaFormat> for AudioProducerStreamPad {
    fn formats(&self) -> Vec<pads::MediaFormat> {
        frequencies_to_s16_pcm(&vec![16000])
    }
}

impl StreamPad<MediaData, MediaFormat> for AudioProducerStreamPad {
    type Receiver = Receiver;

    fn get_rx(
        self,
        format: &pads::MediaFormat,
    ) -> Result<Self::Receiver, pipeline::errors::LinkError> {
        match &format {
            pads::MediaFormat::Audio(AudioFormat::PCM(Pcm {
                layout: Layout::S16LE(_),
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

            let buffer = pads::MediaData::Audio(AudioFormat::PCM(Pcm {
                layout: Layout::S16LE(Tag::new(data)),
                sample_rate: SAMPLE_RATE,
            }));

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            tx.send(buffer).await.unwrap();
        }
    };

    rt.spawn(task);

    AudioProducerStreamPad { rx: Receiver(rx) }
}

struct ConsumerPad {
    tx: Sender,
}

fn consumer_task(rt: &tokio::runtime::Runtime) -> ConsumerPad {
    let (tx, mut rx) = tokio_mpsc::channel(1024);

    let task = async move {
        while let Some(data) = rx.recv().await {
            match &data {
                pads::MediaData::Audio(AudioFormat::PCM(Pcm {
                    layout: Layout::S16LE(data),
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
    fn matches(&self, format: &pads::MediaFormat) -> bool {
        match format {
            pads::MediaFormat::Audio(AudioFormat::PCM(Pcm {
                layout: Layout::S16LE(_),
                sample_rate: 16000,
            })) => true,
            _ => false,
        }
    }
}

impl SinkPad<MediaData, MediaFormat> for ConsumerPad {
    type Sender = Sender;

    fn get_tx(self, _: &pads::MediaFormat) -> Result<Self::Sender, pipeline::errors::LinkError> {
        Ok(self.tx)
    }
}

fn main() {
    use pipeline::pad_element::builder::StreamPadBuilder;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let producer_stream_pad = audio_producer(&rt);

    let passthrough_link = spawn_passthrough_task(&rt);

    let consumer_pad = consumer_task(&rt);

    StreamPadBuilder::with_stream(producer_stream_pad)
        .set_link(&rt, passthrough_link)
        .unwrap()
        .build_with_sink(&rt, consumer_pad)
        .unwrap();

    rt.block_on(async {
        tokio::signal::ctrl_c().await.unwrap();
    });
}
