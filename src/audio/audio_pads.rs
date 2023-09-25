//use crate::pads;
//
//use super::AudioData;
//
//use thingbuf::mpsc as thingbuf_mpsc;
//
//pub trait AudioStreamPad: Send + Sync + Clone + 'static {
//    fn get_format(&self) -> AudioData;
//    fn get_tx(&self) -> thingbuf_mpsc::Sender<Box<[u8]>>;
//}
//
//impl<T: AudioStreamPad> pads::StreamPad<AudioData, Box<[u8]>> for T {
//    fn get_tx(&self) -> thingbuf_mpsc::Sender<Box<[u8]>> {
//        self.get_tx()
//    }
//
//    fn get_format(&self) -> AudioData {
//        self.get_format()
//    }
//}
//
//pub trait AudioSinkPad<C: pads::SinkPadConnected>: Send + Sync + Sized + 'static {
//    fn connect(self, stream: impl AudioStreamPad) -> Result<C, Self>;
//}
//
//impl<T: AudioSinkPad<C>, C: pads::SinkPadConnected> pads::SinkPad<AudioData, Box<[u8]>, C> for T {
//    fn connect(self, stream: impl pads::StreamPad<AudioData, Box<[u8]>>) -> Result<C, Self> {
//        self.connect(stream)
//    }
//}
