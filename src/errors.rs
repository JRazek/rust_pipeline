#[derive(Debug)]
pub enum LinkError {
    InitialFormatMismatch,
    FormatMismatch,
    ChannelClosed,
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LinkError::InitialFormatMismatch => write!(f, "Initial format mismatch"),
            LinkError::FormatMismatch => write!(f, "Format mismatch"),
            LinkError::ChannelClosed => write!(f, "Channel closed"),
        }
    }
}

impl std::error::Error for LinkError {}
