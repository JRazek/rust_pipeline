pub use super::formats::MediaData;
pub use super::formats::MediaFormat;

pub trait NegotiationPad: Send + Sync {
    fn formats(&self) -> Vec<MediaFormat>;
}

pub trait Negotiator: Fn(&MediaFormat) -> bool {}
impl Negotiator for fn(&MediaFormat) -> bool {}

fn test(format: &MediaFormat) -> bool {
    true
}

fn test_negotiator(format: &MediaFormat, negotiator: &impl Negotiator) {
    negotiator(format);
}

pub(super) fn negotiate_formats(
    stream: &impl NegotiationPad,
    sink: &impl NegotiationPad,
) -> Vec<MediaFormat> {
    let stream_format = stream.formats();
    let sink_formats = sink.formats();

    stream_format
        .into_iter()
        .filter(|format| sink_formats.contains(format))
        .collect()
}
