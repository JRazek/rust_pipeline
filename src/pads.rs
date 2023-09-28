pub use super::formats::MediaData;
pub use super::formats::MediaFormat;

pub trait FormatProvider<F>: Send + Sync {
    fn formats(&self) -> Vec<F>;
}

pub trait FormatNegotiator<F>: Send + Sync {
    fn matches(&self, format: &F) -> bool;
}
