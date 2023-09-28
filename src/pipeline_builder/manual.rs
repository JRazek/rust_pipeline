use crate::errors::LinkError;
use crate::pad::*;

use super::*;

pub struct Builder<Stream: StreamPad<Dtx, Ftx>, Dtx, Ftx> {
    pub(super) stream: Stream,
    data_phantom: std::marker::PhantomData<Dtx>,
    format_phantom: std::marker::PhantomData<Ftx>,
}

impl<'a, Stream: StreamPad<D, F>, D: Send + 'static, F: Send + 'static>
    Builder<Stream, D, F>
{
    pub fn with_stream(sink: Stream) -> Self {
        Self {
            stream: sink,
            data_phantom: std::marker::PhantomData,
            format_phantom: std::marker::PhantomData,
        }
    }

    pub fn build_with_sink<T: SinkPad<D, F>>(
        self,
        sink: T,
        format: &F,
        rt: &tokio::runtime::Runtime,
    ) -> Result<(), LinkError> {
        let future = link(self.stream, sink, format)?;

        rt.spawn(wrap_panic_on_err(future));

        Ok(())
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
        LinkT: LinkElement<Drx, Frx, Dtx, Ftx>,
        Dtx: Send + 'static,
        Ftx: Send + 'static,
    >(
        self,
        link_element: LinkT,
        format: &Frx,
        rt: &tokio::runtime::Runtime,
    ) -> Result<Builder<LinkT::StreamPad, Dtx, Ftx>, LinkError>
    where
        LinkT::StreamPad: FormatProvider<Ftx>,
    {
        let (link_sink, link_stream) = link_element.get_pads(format)?;

        let future = link(self.stream, link_sink, format)?;

        rt.spawn(wrap_panic_on_err(future));

        Ok(Builder {
            stream: link_stream,
            data_phantom: std::marker::PhantomData,
            format_phantom: std::marker::PhantomData,
        })
    }
}
