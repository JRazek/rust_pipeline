use crate::errors::LinkError;
use crate::pads::FormatNegotiator;

use super::channel_traits::mpsc::{Receiver, Sender};
use super::pads::FormatProvider;

fn link<D: Send + 'static, F: Send + 'static>(
    stream_pad: impl StreamPad<D, F>,
    sink_pad: impl SinkPad<D, F>,
    format: &F,
) -> Result<impl futures::Future<Output = Result<(), LinkError>> + 'static, LinkError> {
    let mut stream_rx = stream_pad.get_rx(&format)?; //stream_rx is 'static
    let sink_pad = sink_pad.get_tx(&format)?; //sink_pad is 'static

    let task = async move {
        while let Some(data) = stream_rx.recv().await {
            sink_pad
                .send(data)
                .await
                .map_err(|_| LinkError::ChannelClosed)?;
        }

        Ok::<(), LinkError>(())
    };

    Ok(task)
}

pub trait StreamPad<D, F>: Sized {
    type Receiver: Receiver<D>;

    fn get_rx(self, rx_format: &F) -> Result<Self::Receiver, LinkError>;
}

pub trait SinkPad<D, F>: Sized {
    type Sender: Sender<D>;

    fn get_tx(self, tx_format: &F) -> Result<Self::Sender, LinkError>;
}

pub trait LinkElement<Drx, Frx, Dtx = Drx, Ftx = Frx> {
    type SinkPad: SinkPad<Drx, Frx>;
    type StreamPad: StreamPad<Dtx, Ftx> + FormatProvider<Ftx>;

    fn get_pads(self, rx_format: &Frx) -> Result<(Self::SinkPad, Self::StreamPad), LinkError>;
}

fn wrap_panic_on_err<T>(
    result: impl futures::Future<Output = Result<T, LinkError>>,
) -> impl futures::Future<Output = ()> {
    async move {
        if let Err(err) = result.await {
            panic!("Link error: {:?}", err);
        }
    }
}

pub mod builder {
    use super::*;

    pub struct StreamPadBuilder<Stream: StreamPad<Dtx, Ftx>, Dtx, Ftx> {
        stream: Stream,
        formats: Vec<Ftx>,
        data_phantom: std::marker::PhantomData<Dtx>,
        format_phantom: std::marker::PhantomData<Ftx>,
    }

    impl<'a, Stream: StreamPad<D, F> + FormatProvider<F>, D: Send + 'static, F: Send + 'static>
        StreamPadBuilder<Stream, D, F>
    {
        pub fn with_stream(sink: Stream) -> Self {
            let formats = sink.formats();
            Self {
                stream: sink,
                formats,
                data_phantom: std::marker::PhantomData,
                format_phantom: std::marker::PhantomData,
            }
        }

        pub fn build_with_sink<T: SinkPad<D, F> + FormatNegotiator<F>>(
            self,
            sink: T,
            rt: &tokio::runtime::Runtime,
        ) -> Result<(), LinkError> {
            let format = self
                .formats
                .iter()
                .find(|&format| sink.matches(format))
                .ok_or(LinkError::InitialFormatMismatch)?;

            let future = link(self.stream, sink, format)?;

            rt.spawn(wrap_panic_on_err(future));

            Ok(())
        }
    }

    /*
     * Drx, Frx in context of link (Drx, Frx) ---> [(Drx, Frx) ---> (Dtx, Ftx)] ---> (Dtx, Ftx)
     */
    impl<'a, S: StreamPad<Drx, Frx> + 'static, Drx: Send + 'static, Frx: Send + 'static>
        StreamPadBuilder<S, Drx, Frx>
    {
        /*
         * Note that (Sink, Stream) order is reversed here.
         * Link element acts as a bridge between the two pads.
         *
         * ---> [Sink ---> Stream] --->
         *      ^^^^^^Link^^^^^^^^
         *
         */
        pub fn set_link<
            LinkT: LinkElement<Drx, Frx, Dtx, Ftx> + FormatNegotiator<Frx>,
            Dtx: Send + 'static,
            Ftx: Send + 'static,
        >(
            self,
            link_element: LinkT,
            rt: &tokio::runtime::Runtime,
        ) -> Result<StreamPadBuilder<LinkT::StreamPad, Dtx, Ftx>, LinkError> {
            let format = self
                .formats
                .iter()
                .find(|&format| link_element.matches(format))
                .ok_or(LinkError::InitialFormatMismatch)?;

            let (link_sink, link_stream) = link_element.get_pads(format)?;

            let future = link(self.stream, link_sink, format)?;

            rt.spawn(wrap_panic_on_err(future));

            let formats = link_stream.formats();

            Ok(StreamPadBuilder {
                stream: link_stream,
                formats,
                data_phantom: std::marker::PhantomData,
                format_phantom: std::marker::PhantomData,
            })
        }
    }
}
