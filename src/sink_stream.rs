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
>: AsyncLinkWorker<ConsumedType, ProducedType, WorkerReturnType> + 'static
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

pub fn link_channels<T: ChannelType>(rx: Receiver<T>, tx: Sender<T>) -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        while let Some(i) = rx.recv().await {
            tx.send(i).await?;
        }

        Ok(())
    })
}

#[macro_export]
macro_rules! start_and_link_all {
    ($stream:expr, $link:expr, $sink:expr) => {{
        use super::Stream;
        let mut stream = $stream;
        let link = $link;
        stream.link_and_spawn_worker(link);
    }};
    ($stream:expr, $($link:expr),+, $sink:expr) => {{
        let mut stream = $stream;

        let mut sink = $sink;
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

    struct SimpleLink {}

    #[async_trait::async_trait]
    impl AsyncLinkWorker<u32, bool, ()> for SimpleLink {
        async fn run(self, rx: Receiver<u32>, tx: Sender<bool>) -> Result<()> {
            while let Some(i) = rx.recv().await {
                tx.send((i % 2) == 0).await?;
            }

            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Link<u32, bool, ()> for SimpleLink {}

    struct SimpleSink {}

    #[async_trait::async_trait]
    impl AsyncSinkWorker<bool, ()> for SimpleSink {
        async fn run(self, rx: Receiver<bool>) -> Result<()> {
            while let Some(i) = rx.recv().await {
                println!("Sink received: {}", i);
            }

            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Sink<bool, ()> for SimpleSink {}

    #[tokio::test]
    async fn test_simple() {
        let stream = SimpleStream {};
        let link = SimpleLink {};
        let sink = SimpleSink {};

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
    }
}
