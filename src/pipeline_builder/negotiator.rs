use crate::errors::LinkError;
use crate::pad::*;

use super::manual::Builder as ManualBuilder;

use super::manual::CallbackWithFuture;

pub struct Builder<S, D, F>
where
    S: StreamPad<D, F>,
{
    manual_builder: ManualBuilder<S, D, F>,
    formats: Vec<F>,
}

impl<Stream, Dtx, Ftx> Builder<Stream, Dtx, Ftx>
where
    Stream: StreamPad<Dtx, Ftx> + FormatProvider<Ftx>,
    Dtx: Send + 'static,
    Ftx: Send + 'static,
{
    pub fn with_stream(stream: Stream) -> Self {
        let formats = stream.formats();
        let manual_builder = ManualBuilder::with_stream(stream);

        Self {
            formats,
            manual_builder,
        }
    }

    pub fn build_with_sink<T>(self, sink: T, f: &impl CallbackWithFuture) -> Result<(), LinkError>
    where
        T: SinkPad<Dtx, Ftx> + FormatNegotiator<Ftx>,
    {
        let format = self
            .formats
            .iter()
            .find(|&format| sink.matches(format))
            .ok_or(LinkError::InitialFormatMismatch)?;

        self.manual_builder.build_with_sink(sink, format, f)
    }
}

impl<Stream, Drx, Frx> Builder<Stream, Drx, Frx>
where
    Stream: StreamPad<Drx, Frx> + 'static,
    Drx: Send + 'static,
    Frx: Send + 'static,
{
    pub fn set_link<LinkT, Dtx, Ftx>(
        self,
        link_element: LinkT,
        f: &impl CallbackWithFuture,
    ) -> Result<Builder<LinkT::StreamPad, Dtx, Ftx>, LinkError>
    where
        LinkT: LinkElement<Drx, Frx, Dtx, Ftx> + FormatNegotiator<Frx>,
        LinkT::StreamPad: FormatProvider<Ftx>,
        Dtx: Send + 'static,
        Ftx: Send + 'static,
    {
        let format = self
            .formats
            .iter()
            .find(|&format| link_element.matches(format))
            .ok_or(LinkError::InitialFormatMismatch)?;

        let manual_builder = self.manual_builder.set_link(link_element, format, f)?;

        let formats = manual_builder.stream.formats();

        Ok(Builder {
            formats,
            manual_builder,
        })
    }
}

