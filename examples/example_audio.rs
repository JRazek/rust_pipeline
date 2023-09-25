use pipeline::audio;
use pipeline::pads;

use thingbuf::mpsc as thingbuf_mpsc;

use tokio::runtime::Runtime;

#[derive(Clone)]
struct MulawTx {
    pub tx: thingbuf_mpsc::Sender<pads::Data>,
}

impl pads::StreamPad for MulawTx {
    fn get_tx(&self) -> thingbuf_mpsc::Sender<pads::Data> {
        self.tx.clone()
    }

    fn get_format(&self) -> pads::Data {
        pads::Data::Audio(audio::AudioData::Mulaw(audio::Mulaw {
            data: None,
            sample_rate: 8000,
        }))
    }
}

#[derive(Clone)]
struct MulawRx {
    supported_frequencies: Vec<u32>,
}

impl Default for MulawRx {
    fn default() -> Self {
        MulawRx {
            supported_frequencies: vec![8000, 16000],
        }
    }
}

use audio::Mulaw as MulawData;

impl pads::SinkPad<()> for MulawRx {
    fn connect<'a>(
        self,
        stream: impl pads::StreamPad + 'a,
        async_executor: &async_executor::Executor<'a>,
    ) -> Result<(), Self> {
        match stream.get_format() {
            pads::Data::Audio(audio::AudioData::Mulaw(MulawData {
                data: _,
                sample_rate,
            })) => {
                if self.supported_frequencies.contains(&sample_rate) {
                    async_executor.spawn(async move {
                        let mut rx = stream.get_tx();
                        while let Some(data) = rx.recv().await {
                            match data {
                                pads::Data::Audio(audio::AudioData::Mulaw(MulawData {
                                    data,
                                    sample_rate,
                                })) => {
                                    println!("Got mulaw data with sample rate {}", sample_rate);
                                }
                                _ => {}
                            }
                        }
                    });
                    Ok(())
                } else {
                    Err(self)
                }
            }
            _ => Err(self),
        }
    }
}

fn main() {}
