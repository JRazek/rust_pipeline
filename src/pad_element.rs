use crate::errors::LinkError;
use crate::formats::{MediaData, MediaFormat};

use super::channel_traits::mpsc::{Receiver, Sender};
use super::pads::{negotiate_formats, NegotiationPad};

fn link(
    stream_pad: impl StreamPad,
    sink_pad: impl SinkPad,
) -> Result<impl futures::Future<Output = Result<(), LinkError>>, LinkError> {
    let negotiated = negotiate_formats(&stream_pad, &sink_pad);

    let format = match &negotiated[..] {
        [format, ..] => Some(format),
        _ => None,
    };

    match format {
        Some(format) => {
            let mut stream_pad = stream_pad.get_tx(&format)?;
            let sink_pad = sink_pad.get_rx(&format)?;

            let task = async move {
                while let Some(data) = stream_pad.recv().await {
                    sink_pad
                        .send(data)
                        .await
                        .map_err(|_| LinkError::ChannelClosed)?;
                }

                Ok::<(), LinkError>(())
            };

            Ok(task)
        }
        None => Err(LinkError::InitialFormatMismatch),
    }
}

pub trait StreamPad: Sized + NegotiationPad {
    type Receiver: Receiver<MediaData>;

    fn get_tx(self, format: &MediaFormat) -> Result<Self::Receiver, LinkError>;
}

pub trait SinkPad: Sized + NegotiationPad {
    type Sender: Sender<MediaData>;

    fn get_rx(self, format: &MediaFormat) -> Result<Self::Sender, LinkError>;
}

pub mod builder {
    use super::*;

    pub struct StreamPadBuilder<'a, S: StreamPad> {
        stream: S,
        async_executor: async_executor::Executor<'a>,
    }

    impl<'a, S: StreamPad + 'a> StreamPadBuilder<'a, S> {
        pub fn with_stream(sink: S) -> Self {
            Self {
                stream: sink,
                async_executor: async_executor::Executor::new(),
            }
        }

        pub fn set_sink<T: SinkPad + 'a>(self, sink: T) -> Result<(), LinkError> {
            let future = link(self.stream, sink)?;
            self.async_executor.spawn(future).detach();

            Ok(())
        }

        /*
         * Note that (Sink, Stream) order is reversed here.
         * Link element acts as a bridge between the two pads.
         *
         * ---> [Sink ---> Stream] --->
         *      ^^^^^^Link^^^^^^^^
         *
         */
        pub fn set_link<Sink: SinkPad + 'a, Stream: StreamPad + 'a>(
            self,
            link_tup: (Sink, Stream),
        ) -> Result<StreamPadBuilder<'a, Stream>, LinkError> {
            let (sink, stream) = link_tup;

            let link = link(self.stream, sink)?;

            self.async_executor.spawn(link).detach();

            Ok(StreamPadBuilder {
                stream,
                async_executor: self.async_executor,
            })
        }
    }
}
