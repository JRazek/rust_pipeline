use crate::errors::LinkError;
use crate::pad::*;

use super::manual::Builder as ManualBuilder;

pub struct Builder<Stream: StreamPad<Dtx, Ftx>, Dtx, Ftx> {
    manual_builder: ManualBuilder<Stream, Dtx, Ftx>,
    formats: Vec<Ftx>,
}

impl<'a, Stream: StreamPad<D, F> + FormatProvider<F>, D: Send + 'static, F: Send + 'static>
    Builder<Stream, D, F>
{
    pub fn with_stream(sink: Stream) -> Self {
        let formats = sink.formats();

        let manual_builder = ManualBuilder::with_stream(sink);

        Self {
            formats,
            manual_builder,
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

        self.manual_builder.build_with_sink(sink, format, rt)
    }
}

/*
 * Drx, Frx in context of link (Drx, Frx) ---> [(Drx, Frx) ---> (Dtx, Ftx)] ---> (Dtx, Ftx)
 */
impl<'a, S: StreamPad<Drx, Frx> + 'static, Drx: Send + 'static, Frx: Send + 'static>
    Builder<S, Drx, Frx>
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
    ) -> Result<Builder<LinkT::StreamPad, Dtx, Ftx>, LinkError>
    where
        LinkT::StreamPad: FormatProvider<Ftx>,
    {
        let format = self
            .formats
            .iter()
            .find(|&format| link_element.matches(format))
            .ok_or(LinkError::InitialFormatMismatch)?;

        let manual_builder = self.manual_builder.set_link(link_element, format, rt)?;

        let formats = manual_builder.stream.formats();

        Ok(Builder {
            formats,
            manual_builder,
        })
    }
}
