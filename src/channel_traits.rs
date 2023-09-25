pub mod mpsc {
    use std::future::Future;

    pub trait Receiver<I>: Send + Sync {
        type Item;
        fn recv(&mut self) -> Box<dyn Future<Output = Option<I>> + Sync + Send + Unpin>;
    }

    #[derive(Debug)]
    pub struct SendError<I>(pub I);

    impl std::fmt::Display for SendError<()> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "")
        }
    }

    impl std::error::Error for SendError<()> {}

    pub trait Sender<I>: Send + Sync {
        type Item;
        fn send(
            &self,
            item: I,
        ) -> Box<dyn Future<Output = Result<(), SendError<I>>> + Sync + Send + Unpin>;
    }
}

pub mod oneshot {
    use std::future::Future;

    pub trait Receiver<I, E>: Future<Output = Result<I, E>> + Send + Sync + Unpin {}

    pub trait Sender<I, E>: Send + Sync + Unpin {
        fn send(self, item: I) -> Box<dyn Future<Output = Result<I, E>> + Send + Sync + Unpin>;
    }
}
