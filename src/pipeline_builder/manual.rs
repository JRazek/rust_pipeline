use crate::errors::LinkError;
use crate::pad::*;

use super::*;

pub struct Builder<Stream, Dtx, Ftx>
where
    Stream: StreamPad<Dtx, Ftx>,
{
    pub(super) stream: Stream,
    data_phantom: std::marker::PhantomData<Dtx>,
    format_phantom: std::marker::PhantomData<Ftx>,
}

impl<'a, Stream, Data, Format> Builder<Stream, Data, Format>
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
impl<'a, S, Drx, Frx> Builder<S, Drx, Frx>
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
        rt: &tokio::runtime::Runtime,
    ) -> Result<Builder<LinkT::StreamPad, Dtx, Ftx>, LinkError>
    where
        LinkT: LinkElement<Drx, Frx, Dtx, Ftx>,
        Dtx: Send + 'static,
        Ftx: Send + 'static,
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

//fn test<I, E, F: futures::Future<Output = Result<I, E>>>(
//) -> impl FnOnce(F) -> () + Send + 'static {
//}
