pub use super::formats::MediaData;
pub use super::formats::MediaFormat;

pub trait FormatProvider<F>: Send + Sync {
    fn formats(&self) -> Vec<F>;
}

pub trait FormatNegotiator<F>: Send + Sync {
    fn matches(&self, format: &F) -> bool;
}

pub(super) fn negotiate_formats<F>(
    stream: &impl FormatProvider<F>,
    sink: &impl FormatNegotiator<F>,
) -> Vec<F> {
    let stream_format = stream.formats();

    stream_format
        .into_iter()
        .filter(|format| sink.matches(format))
        .collect()
}
