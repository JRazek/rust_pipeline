pub use super::formats::MediaData;
pub use super::formats::MediaFormat;

pub trait FormatProvider: Send + Sync {
    fn formats(&self) -> Vec<MediaFormat>;
}

pub trait FormatNegotiator: Send + Sync {
    fn matches(&self, format: &MediaFormat) -> bool;
}

pub(super) fn negotiate_formats(
    stream: &impl FormatProvider,
    sink: &impl FormatNegotiator,
) -> Vec<MediaFormat> {
    let stream_format = stream.formats();

    stream_format
        .into_iter()
        .filter(|format| sink.matches(format))
        .collect()
}
