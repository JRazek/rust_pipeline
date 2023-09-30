use crate::errors::LinkError;
use crate::pad::*;

use crate::future_utils::CallbackWithFuture;

pub struct Builder<Stream, Dtx, Ftx>
where
    Stream: StreamPad<Dtx, Ftx>,
{
    pub(super) stream: Stream,
    data_phantom: std::marker::PhantomData<Dtx>,
    format_phantom: std::marker::PhantomData<Ftx>,
}

impl<Stream, Data, Format> Builder<Stream, Data, Format>
where
    Stream: StreamPad<Data, Format>,
    Data: Send + 'static,
    Format: Send + 'static,
{
    pub fn with_stream(sink: Stream) -> Self {
        Self {
            stream: sink,
            data_phantom: std::marker::PhantomData,
            format_phantom: std::marker::PhantomData,
        }
    }

    pub fn build_with_sink<T: SinkPad<Data, Format>>(
        self,
        sink: T,
        format: &Format,
        f: impl CallbackWithFuture<Result<(), LinkError>>,
    ) -> Result<(), LinkError> {
        let future = link(self.stream, sink, format)?;

        f.call(Box::pin(future));

        Ok(())
    }
}

/*
 * Drx, Frx in context of link (Drx, Frx) ---> [(Drx, Frx) ---> (Dtx, Ftx)] ---> (Dtx, Ftx)
 */
impl<S, Drx, Frx> Builder<S, Drx, Frx>
where
    S: StreamPad<Drx, Frx> + 'static,
    Drx: Send + 'static,
    Frx: Send + 'static,
{
    /*
     * Note that (Sink, Stream) order is reversed here.
     * Link element acts as a bridge between the two pads.
     *
     * ---> [Sink ---> Stream] --->
     *      ^^^^^^Link^^^^^^^^
     *
     */
    pub fn set_link<LinkT, Dtx, Ftx>(
        self,
        link_element: LinkT,
        format: &Frx,
        f: impl CallbackWithFuture<Result<(), LinkError>>,
    ) -> Result<Builder<LinkT::StreamPad, Dtx, Ftx>, LinkError>
    where
        LinkT: LinkElement<Drx, Frx, Dtx, Ftx>,
        Dtx: Send + 'static,
        Ftx: Send + 'static,
    {
        let (link_sink, link_stream) = link_element.get_pads(format)?;

        let future = link(self.stream, link_sink, format)?;

        f.call(Box::pin(future));

        Ok(Builder {
            stream: link_stream,
            data_phantom: std::marker::PhantomData,
            format_phantom: std::marker::PhantomData,
        })
    }
}
