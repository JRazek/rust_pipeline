use super::pad::*;
use crate::errors::LinkError;

fn wrap_panic_on_err<T>(
    result: impl futures::Future<Output = Result<T, LinkError>>,
) -> impl futures::Future<Output = ()> {
    async move {
        if let Err(err) = result.await {
            panic!("Link error: {:?}", err);
        }
    }
}

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
