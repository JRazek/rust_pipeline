use crate::formats::MediaData;

use super::channel_traits::mpsc;
use super::errors::LinkError;

pub use super::formats::MediaFormat;

pub trait NegotiableStreamPad<R: mpsc::Receiver<MediaData>>: Send + Sync {
    fn matches_format(&self) -> Vec<MediaFormat>;
}

pub trait NegotiableSinkPad<S: mpsc::Sender<MediaData>>: Send + Sync + Sized {
    fn matches_format(&self) -> Vec<MediaFormat>;
}

pub(super) fn negotiate_formats<R: mpsc::Receiver<MediaData>, S: mpsc::Sender<MediaData>>(
    stream: &impl NegotiableStreamPad<R>,
    sink: &impl NegotiableSinkPad<S>,
) -> Vec<MediaFormat> {
    let stream_format = stream.matches_format();
    let sink_formats = sink.matches_format();

    stream_format
        .into_iter()
        .filter(|format| sink_formats.contains(format))
        .collect()
}
