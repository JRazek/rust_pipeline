use crate::channel_traits::mpsc::{Receiver, Sender};
use crate::errors::LinkError;

pub fn link<D: Send + 'static, F: Send + 'static>(
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

pub trait FormatProvider<F> {
    fn formats(&self) -> Vec<F>;
}

pub trait FormatNegotiator<F> {
    fn matches(&self, format: &F) -> bool;
}

pub trait StreamPad<D, F> {
    type Receiver: Receiver<D>;

    fn get_rx(self, rx_format: &F) -> Result<Self::Receiver, LinkError>;
}

pub trait SinkPad<D, F> {
    type Sender: Sender<D>;

    fn get_tx(self, tx_format: &F) -> Result<Self::Sender, LinkError>;
}

pub trait LinkElement<Drx, Frx, Dtx = Drx, Ftx = Frx> {
    type SinkPad: SinkPad<Drx, Frx>;
    type StreamPad: StreamPad<Dtx, Ftx>;

    fn get_pads(self, rx_format: &Frx) -> Result<(Self::SinkPad, Self::StreamPad), LinkError>;
}
