use crate::errors::LinkError;
use crate::formats::{MediaData, MediaFormat};
use crate::pads::FormatNegotiator;

use super::channel_traits::mpsc::{Receiver, Sender};
use super::pads::{negotiate_formats, FormatProvider};

fn link<D: Send, F: Send>(
    stream_pad: impl StreamPad<D, F>,
    sink_pad: impl SinkPad<D, F>,
) -> Result<impl futures::Future<Output = Result<(), LinkError>>, LinkError> {
    let negotiated = negotiate_formats(&stream_pad, &sink_pad);

    let format = match &negotiated[..] {
        [format, ..] => Some(format),
        _ => None,
    };

    match format {
        Some(format) => {
            let mut stream_pad = stream_pad.get_rx(&format)?;
            let sink_pad = sink_pad.get_tx(&format)?;

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

pub trait StreamPad<D, F>: Sized + FormatProvider<F> {
    type Receiver: Receiver<D>;

    fn get_rx(self, format: &F) -> Result<Self::Receiver, LinkError>;
}

pub trait SinkPad<D, F>: Sized + FormatNegotiator<F> {
    type Sender: Sender<D>;

    fn get_tx(self, format: &F) -> Result<Self::Sender, LinkError>;
}

pub mod builder {
    use super::*;

    pub struct StreamPadBuilder<'a, S: StreamPad<D, F>, D, F> {
        stream: S,
        async_executor: async_executor::Executor<'a>,
        data_phantom: std::marker::PhantomData<D>,
        format_phantom: std::marker::PhantomData<F>,
    }

    impl<'a, S: StreamPad<D, F> + 'a, D: Send + 'a, F: Send + 'a> StreamPadBuilder<'a, S, D, F> {
        pub fn with_stream(sink: S) -> Self {
            Self {
                stream: sink,
                async_executor: async_executor::Executor::new(),
                data_phantom: std::marker::PhantomData,
                format_phantom: std::marker::PhantomData,
            }
        }

        pub fn build_with_sink<T: SinkPad<D, F> + 'a>(self, sink: T) -> Result<(), LinkError> {
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
        pub fn set_link<Sink: SinkPad<D, F> + 'a, Stream: StreamPad<D, F> + 'a>(
            self,
            link_tup: (Sink, Stream),
        ) -> Result<StreamPadBuilder<'a, Stream, D, F>, LinkError> {
            let (sink, stream) = link_tup;

            let link = link(self.stream, sink)?;

            self.async_executor.spawn(link).detach();

            Ok(StreamPadBuilder {
                stream,
                async_executor: self.async_executor,
                data_phantom: std::marker::PhantomData,
                format_phantom: std::marker::PhantomData,
            })
        }
    }
}
