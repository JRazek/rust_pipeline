use pipeline::audio::pcm::{Layout, Pcm};
use pipeline::audio::AudioFormat;
use pipeline::pad_element::{SinkPad, StreamPad};
use pipeline::pads::{self, FormatNegotiator, FormatProvider, MediaFormat};
use pipeline::tags::Tag;

use tokio::sync::mpsc as thingbuf_mpsc;
use tokio::sync::oneshot as tokio_oneshot;

use pipeline::channel_traits::mpsc as pipeline_mpsc;

#[derive(Clone)]
pub struct Sender(pub thingbuf_mpsc::Sender<pads::MediaData>);

pub struct Receiver(pub thingbuf_mpsc::Receiver<pads::MediaData>);

#[async_trait::async_trait]
impl pipeline_mpsc::Sender<pads::MediaData> for Sender {
    async fn send(
        &self,
        data: pads::MediaData,
    ) -> Result<(), pipeline_mpsc::SendError<pads::MediaData>> {
        let tx = &self.0;

        tx.send(data)
            .await
            .map_err(|thingbuf_mpsc::error::SendError(data)| pipeline_mpsc::SendError(data))
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
    format_tx: tokio_oneshot::Sender<MediaFormat>,
}

impl FormatNegotiator for PassthroughSinkPad {
    fn matches(&self, _: &pads::MediaFormat) -> bool {
        true
    }
}

impl SinkPad for PassthroughSinkPad {
    type Sender = Sender;

    fn get_tx(self, _: &pads::MediaFormat) -> Result<Self::Sender, pipeline::errors::LinkError> {
        Ok(self.tx)
    }
}

struct PassthroughStreamPad {
    supported_frequencies: Vec<u32>,
    rx: Receiver,
    format_rx: tokio_oneshot::Receiver<MediaFormat>,
}

impl StreamPad for PassthroughStreamPad {
    type Receiver = Receiver;

    fn get_rx(
        self,
        format: &pads::MediaFormat,
    ) -> Result<Self::Receiver, pipeline::errors::LinkError> {
        Ok(self.rx)
    }
}

impl FormatProvider for PassthroughStreamPad {
    fn formats(&self) -> Vec<pads::MediaFormat> {
        frequencies_to_s16_pcm(&self.supported_frequencies)
    }
}

fn spawn_passthrough_task(
    async_executor: &async_executor::Executor,
) -> (PassthroughSinkPad, PassthroughStreamPad) {
    let (in_tx, in_rx) = thingbuf_mpsc::channel(1024);
    let (out_tx, out_rx) = thingbuf_mpsc::channel(1024);

    let (format_tx, format_rx) = tokio_oneshot::channel();

    async_executor
        .spawn(passthrough_task(Receiver(in_rx), Sender(out_tx)))
        .detach();

    (
        PassthroughSinkPad {
            tx: Sender(in_tx),
            format_tx,
        },
        PassthroughStreamPad {
            supported_frequencies: vec![8000],
            format_rx,
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
}

fn signal(time: std::time::Duration) -> i16 {
    let time = time.as_millis() as f64;

    let converted = (time.sin() * i16::MAX as f64) as i16;

    converted
}

struct AudioProducerStreamPad {
    rx: Receiver,
}

impl FormatProvider for AudioProducerStreamPad {
    fn formats(&self) -> Vec<pads::MediaFormat> {
        frequencies_to_s16_pcm(&vec![8000])
    }
}

impl StreamPad for AudioProducerStreamPad {
    type Receiver = Receiver;

    fn get_rx(
        self,
        format: &pads::MediaFormat,
    ) -> Result<Self::Receiver, pipeline::errors::LinkError> {
        match &format {
            pads::MediaFormat::Audio(AudioFormat::PCM(Pcm {
                layout: Layout::S16LE(_),
                sample_rate: 8000,
            })) => {
                let rx = self.rx;

                Ok(rx)
            }
            _ => Err(pipeline::errors::LinkError::InitialFormatMismatch),
        }
    }
}

fn audio_producer(async_executor: &async_executor::Executor) -> AudioProducerStreamPad {
    const SAMPLE_RATE: u32 = 8000;

    let sample_duration =
        std::time::Duration::from_millis((1. / SAMPLE_RATE as f64 * 1000.) as u64);

    let mut time = std::time::Duration::ZERO;

    let (tx, rx) = thingbuf_mpsc::channel(1024);

    let task = async move {
        loop {
            let mut buffer = vec![0i16; 1024];

            for i in 0..buffer.len() {
                buffer[i] = signal(time);

                time += sample_duration;
            }

            let data = buffer.into_boxed_slice();

            let buffer = pads::MediaData::Audio(AudioFormat::PCM(Pcm {
                layout: Layout::S16LE(Tag::new(data)),
                sample_rate: SAMPLE_RATE,
            }));

            tx.send(buffer).await.unwrap();
        }
    };
    async_executor.spawn(task).detach();

    AudioProducerStreamPad { rx: Receiver(rx) }
}

struct ConsumerPad {
    tx: Sender,
}

fn consumer_task(async_executor: &async_executor::Executor) -> ConsumerPad {
    let (tx, mut rx) = thingbuf_mpsc::channel(1024);

    let task = async move {
        while let Some(data) = rx.recv().await {
            match &data {
                pads::MediaData::Audio(AudioFormat::PCM(Pcm {
                    layout: Layout::S16LE(data),
                    sample_rate: 8000,
                })) => {
                    println!("Got audio data: {:?}", data);
                }
                _ => {
                    panic!("Got unexpected data");
                }
            }
        }
    };

    async_executor.spawn(task).detach();

    ConsumerPad { tx: Sender(tx) }
}

impl FormatNegotiator for ConsumerPad {
    fn matches(&self, format: &pads::MediaFormat) -> bool {
        match format {
            pads::MediaFormat::Audio(AudioFormat::PCM(Pcm {
                layout: Layout::S16LE(_),
                sample_rate: 8000,
            })) => true,
            _ => false,
        }
    }
}

impl SinkPad for ConsumerPad {
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

    let async_executor = async_executor::Executor::new();

    let producer_stream_pad = audio_producer(&async_executor);

    let passthrough_link = spawn_passthrough_task(&async_executor);

    let consumer_pad = consumer_task(&async_executor);

    StreamPadBuilder::with_stream(producer_stream_pad)
        .set_link(passthrough_link)
        .unwrap()
        .build_with_sink(consumer_pad)
        .unwrap();

    let runner = rt.spawn(async move {
        loop {
            async_executor.tick().await;
        }
    });

    rt.block_on(runner).unwrap();
}
