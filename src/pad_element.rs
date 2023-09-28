use crate::errors::LinkError;
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

async fn wrap_panic_on_err<T>(result: impl futures::Future<Output = Result<T, LinkError>>) {
    if let Err(err) = result.await {
        panic!("Link error: {:?}", err);
    }
}

pub mod builder {
    use super::*;

    pub struct StreamPadBuilder<S: StreamPad<D, F> + 'static, D, F> {
        stream: S,
        data_phantom: std::marker::PhantomData<D>,
        format_phantom: std::marker::PhantomData<F>,
    }

    impl<'a, S: StreamPad<D, F> + 'static, D: Send + 'static, F: Send + 'static>
        StreamPadBuilder<S, D, F>
    {
        pub fn with_stream(sink: S) -> Self {
            Self {
                stream: sink,
                data_phantom: std::marker::PhantomData,
                format_phantom: std::marker::PhantomData,
            }
        }

        pub fn build_with_sink<T: SinkPad<D, F> + 'static>(
            self,
            rt: &tokio::runtime::Runtime,
            sink: T,
        ) -> Result<(), LinkError> {
            let future = link(self.stream, sink)?;

            rt.spawn(wrap_panic_on_err(future));

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
        pub fn set_link<Sink: SinkPad<D, F> + 'static, Stream: StreamPad<D, F> + 'static>(
            self,
            rt: &tokio::runtime::Runtime,
            link_tup: (Sink, Stream),
        ) -> Result<StreamPadBuilder<Stream, D, F>, LinkError> {
            let (sink, stream) = link_tup;

            let future = link(self.stream, sink)?;

            rt.spawn(wrap_panic_on_err(future));

            Ok(StreamPadBuilder {
                stream,
                data_phantom: std::marker::PhantomData,
                format_phantom: std::marker::PhantomData,
            })
        }
    }
}
