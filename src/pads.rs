use super::audio::AudioData;
use thingbuf::mpsc as thingbuf_mpsc;

pub enum Data {
    Audio(AudioData),
}

pub trait StreamPad: Send + Sync + Clone {
    //cant be tx??
    fn get_tx(&self) -> thingbuf_mpsc::Sender<Data>;
    fn get_format(&self) -> Data;
}

pub trait SinkPad<C>: Send + Sync + Sized {
    fn connect<'a>(
        self,
        stream: impl StreamPad + 'a,
        async_executor: &async_executor::Executor<'a>,
    ) -> Result<C, Self>;
}
