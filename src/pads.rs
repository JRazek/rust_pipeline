use super::channel_traits::mpsc;

pub use super::formats::TaggedData;

pub trait NegotiableStreamPad<R: mpsc::Receiver<TaggedData>>: Send + Sync + Clone {
    fn to_stream_pad(self, format: &TaggedData) -> Result<R, LinkError>;

    fn formats(&self) -> Vec<TaggedData>;
}

pub trait NegotiableSinkPad<S: mpsc::Sender<TaggedData>>: Send + Sync + Sized {
    fn to_sink_pad(self, format: &TaggedData) -> Result<S, LinkError>;

    fn formats(&self) -> Vec<TaggedData>;
}

use super::errors::LinkError;

fn negotiate_formats<R: mpsc::Receiver<TaggedData>, S: mpsc::Sender<TaggedData>>(
    stream: &impl NegotiableStreamPad<R>,
    sink: &impl NegotiableSinkPad<S>,
) -> Vec<TaggedData> {
    let stream_format = stream.formats();
    let sink_formats = sink.formats();

    stream_format
        .into_iter()
        .filter(|format| sink_formats.contains(format))
        .collect()
}

pub async fn link_pads<'a, R: mpsc::Receiver<TaggedData>, S: mpsc::Sender<TaggedData>>(
    stream: impl NegotiableStreamPad<R> + 'a,
    sink: impl NegotiableSinkPad<S> + 'a,
) -> Result<(), LinkError> {
    let negotiated = negotiate_formats(&stream, &sink);

    let format = match &negotiated[..] {
        [format, ..] => Some(format.clone()),
        _ => None,
    };

    match format {
        Some(format) => {
            let stream_pad = stream.to_stream_pad(&format)?;
            let sink_pad = sink.to_sink_pad(&format)?;

            let task = pads_worker(stream_pad, sink_pad).await;

            task
        }
        None => Err(LinkError::InitialFormatMismatch),
    }
}

pub async fn pads_worker<R: mpsc::Receiver<TaggedData>, S: mpsc::Sender<TaggedData>>(
    mut stream_pad: R,
    sink: S,
) -> Result<(), LinkError> {
    let task = async move {
        while let Some(data) = stream_pad.recv().await {
            sink.send(data)
                .await
                .map_err(|_| LinkError::ChannelClosed)?;
        }

        Ok::<(), LinkError>(())
    };

    task.await
}
