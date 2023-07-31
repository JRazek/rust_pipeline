use std::marker::PhantomData;
use tokio::task::JoinHandle;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

use thingbuf::mpsc::{Receiver, Sender};

//TODO when stable rust: trait ChannelType = Send + Sync + Default + 'static;
pub trait ChannelType: Send + Sync + Clone + Default + 'static {}
impl<T: Send + Sync + Clone + Default + 'static> ChannelType for T {}

#[async_trait::async_trait]
pub trait AsyncSinkWorker<T: ChannelType, ReturnType> {
    async fn run(self, rx: Receiver<T>) -> Result<ReturnType>;
}

#[async_trait::async_trait]
pub trait Sink<T: ChannelType, WorkerReturnType: Send + 'static>:
    AsyncSinkWorker<T, WorkerReturnType> + Send + Sync + 'static
{
    async fn start(self, channel_size: usize) -> (JoinHandle<Result<WorkerReturnType>>, Sender<T>)
    where
        Self: Sized,
        T: ChannelType,
    {
        let (tx, rx) = thingbuf::mpsc::channel::<T>(channel_size);

        let join_handle = tokio::spawn(self.run(rx));

        (join_handle, tx)
    }
}

#[async_trait::async_trait]
pub trait AsyncStreamWorker<T: ChannelType, ReturnType> {
    async fn run(self, tx: Sender<T>) -> Result<ReturnType>;
}

#[async_trait::async_trait]
pub trait Stream<T: ChannelType, WorkerReturnType: Send + 'static>:
    AsyncStreamWorker<T, WorkerReturnType> + Send + Sync + 'static
{
    async fn start(self, channel_size: usize) -> (JoinHandle<Result<WorkerReturnType>>, Receiver<T>)
    where
        Self: Sized,
        T: ChannelType,
    {
        let (tx, rx) = thingbuf::mpsc::channel::<T>(channel_size);

        let join_handle = tokio::spawn(self.run(tx));

        (join_handle, rx)
    }
}

#[async_trait::async_trait]
pub trait AsyncLinkWorker<ConsumedType: ChannelType, ProducedType: ChannelType, ReturnType> {
    async fn run(self, rx: Receiver<ConsumedType>, tx: Sender<ProducedType>) -> Result<ReturnType>;
}

#[async_trait::async_trait]
pub trait Link<
    ConsumedType: ChannelType,
    ProducedType: ChannelType,
    WorkerReturnType: Send + 'static,
>: Send + AsyncLinkWorker<ConsumedType, ProducedType, WorkerReturnType> + 'static
{
    async fn start(
        self,
        channel_size: usize,
    ) -> (
        JoinHandle<Result<WorkerReturnType>>,
        Sender<ConsumedType>,
        Receiver<ProducedType>,
    )
    where
        Self: Sized,
        ConsumedType: ChannelType,
        ProducedType: ChannelType,
    {
        let (inner_tx, inner_rx) = thingbuf::mpsc::channel::<ConsumedType>(channel_size);
        let (outer_tx, outer_rx) = thingbuf::mpsc::channel::<ProducedType>(channel_size);

        let join_handle = tokio::spawn(self.run(inner_rx, outer_tx));

        (join_handle, inner_tx, outer_rx)
    }
}

pub struct LinkWrapper<
    InputType: ChannelType,
    OutputType: ChannelType,
    CommunicationType: ChannelType,
    Link1: Link<InputType, CommunicationType, ()>,
    Link2: Link<CommunicationType, OutputType, ()>,
> {
    _phantom: PhantomData<InputType>,
    _phantom2: PhantomData<OutputType>,
    _phantom3: PhantomData<CommunicationType>,

    link1: Link1,
    link2: Link2,
}

impl<
        InputType: ChannelType,
        OutputType: ChannelType,
        CommunicationType: ChannelType,
        Link1: Link<InputType, CommunicationType, ()>,
        Link2: Link<CommunicationType, OutputType, ()>,
    > LinkWrapper<InputType, OutputType, CommunicationType, Link1, Link2>
{
    #[allow(dead_code)]
    pub fn new(link1: Link1, link2: Link2) -> Self {
        Self {
            _phantom: PhantomData,
            _phantom2: PhantomData,
            _phantom3: PhantomData,
            link1,
            link2,
        }
    }
}

#[async_trait::async_trait]
impl<
        InputType: ChannelType,
        OutputType: ChannelType,
        CommunicationType: ChannelType,
        Link1: Link<InputType, CommunicationType, ()>,
        Link2: Link<CommunicationType, OutputType, ()>,
    > AsyncLinkWorker<InputType, OutputType, ()>
    for LinkWrapper<InputType, OutputType, CommunicationType, Link1, Link2>
{
    async fn run(self, rx: Receiver<InputType>, tx: Sender<OutputType>) -> Result<()> {
        let (link1_join_handle, link1_tx, link1_rx) = self.link1.start(10).await;
        let (link2_join_handle, link2_tx, link2_rx) = self.link2.start(10).await;

        let link_in = link_channels(rx, link1_tx);
        let link_out = link_channels(link2_rx, tx);

        let link_internal = link_channels(link1_rx, link2_tx);

        let _ = tokio::join!(
            link1_join_handle,
            link2_join_handle,
            link_in,
            link_out,
            link_internal
        );

        Ok(())
    }
}

#[async_trait::async_trait]
impl<
        InputType: ChannelType,
        OutputType: ChannelType,
        CommunicationType: ChannelType,
        Link1: Link<InputType, CommunicationType, ()>,
        Link2: Link<CommunicationType, OutputType, ()>,
    > Link<InputType, OutputType, ()>
    for LinkWrapper<InputType, OutputType, CommunicationType, Link1, Link2>
{
}

pub fn link_channels<T: ChannelType>(rx: Receiver<T>, tx: Sender<T>) -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        while let Some(i) = rx.recv().await {
            tx.send(i).await?;
        }

        Ok(())
    })
}

#[macro_export]
macro_rules! eval_links {
    ($link:expr) => {
        $link
    };

    ($link:expr, $($links:expr),+) => {
        LinkWrapper::new($link, eval_links!($($links),*))
    };
}

#[macro_export]
macro_rules! start_and_link_all {
    ($stream:expr, $($link:expr);+, $sink:expr) => {{
        let stream = $stream;
        let link = eval_links!($($link),+);
        let sink = $sink;

        let (stream_join_handle, stream_rx) = stream.start(10).await;
        let (link_join_handle, link_tx, link_rx) = link.start(10).await;
        let (sink_join_handle, sink_tx) = sink.start(10).await;

        let link1 = link_channels(stream_rx, link_tx);
        let link2 = link_channels(link_rx, sink_tx);

        let _ = tokio::join!(
            stream_join_handle,
            link_join_handle,
            sink_join_handle,
            link1,
            link2
        );
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SimpleStream {}

    #[async_trait::async_trait]
    impl AsyncStreamWorker<u32, ()> for SimpleStream {
        async fn run(self, tx: Sender<u32>) -> Result<()> {
            for i in 0..10 {
                tx.send(i).await?;
            }

            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Stream<u32, ()> for SimpleStream {}

    struct X3 {}

    #[async_trait::async_trait]
    impl AsyncLinkWorker<u32, u32, ()> for X3 {
        async fn run(self, rx: Receiver<u32>, tx: Sender<u32>) -> Result<()> {
            while let Some(i) = rx.recv().await {
                tx.send(i * 3).await?;
            }

            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Link<u32, u32, ()> for X3 {}

    struct X10 {}

    #[async_trait::async_trait]
    impl AsyncLinkWorker<u32, u32, ()> for X10 {
        async fn run(self, rx: Receiver<u32>, tx: Sender<u32>) -> Result<()> {
            while let Some(i) = rx.recv().await {
                tx.send(i * 10).await?;
            }

            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Link<u32, u32, ()> for X10 {}

    struct Div2 {}

    #[async_trait::async_trait]
    impl AsyncLinkWorker<u32, u32, ()> for Div2 {
        async fn run(self, rx: Receiver<u32>, tx: Sender<u32>) -> Result<()> {
            while let Some(i) = rx.recv().await {
                tx.send(i / 2).await?;
            }

            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Link<u32, u32, ()> for Div2 {}

    struct SimpleSink {}

    #[async_trait::async_trait]
    impl AsyncSinkWorker<u32, ()> for SimpleSink {
        async fn run(self, rx: Receiver<u32>) -> Result<()> {
            let mut n = 0;

            while let Some(i) = rx.recv().await {
                assert_eq!(i, n * 15);
                n += 1;
            }

            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Sink<u32, ()> for SimpleSink {}

    #[tokio::test]
    async fn test_macro() {
        let stream = SimpleStream {};
        let x3 = X3 {};
        let x10 = X10 {};
        let div2 = Div2 {};

        let link = eval_links!(x3, x10, div2);

        let sink = SimpleSink {};

        start_and_link_all!(stream, link, sink);
    }
}
