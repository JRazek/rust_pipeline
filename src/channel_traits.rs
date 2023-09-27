pub mod mpsc {
    #[async_trait::async_trait]
    pub trait Receiver<I>: Send + Sync {
        async fn recv(&mut self) -> Option<I>;
    }

    #[derive(Debug)]
    pub struct SendError<I>(pub I);

    impl std::fmt::Display for SendError<()> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "")
        }
    }

    impl std::error::Error for SendError<()> {}

    #[async_trait::async_trait]
    pub trait Sender<I>: Send + Sync {
        async fn send(&self, item: I) -> Result<(), SendError<I>>;
    }
}
