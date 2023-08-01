use super::channel_utils::{branch_channels, ChannelType};
use std::marker::PhantomData;
use tokio::{sync::oneshot, task::JoinHandle};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

use thingbuf::mpsc::{Receiver as ThingbufReceiver, Sender as ThingbufSender};

//TODO when stable rust: trait ChannelType = Send + Sync + Default + 'static;
impl<T: Send + Sync + Clone + Default + 'static> ChannelType for T {}

#[async_trait::async_trait]
pub trait AsyncSinkWorker<T: ChannelType, ReturnType> {
    async fn run(
        self,
        rx: ThingbufReceiver<T>,
        oneshot: oneshot::Receiver<()>,
    ) -> Result<ReturnType>;
}

#[async_trait::async_trait]
pub trait Sink<T: ChannelType, WorkerReturnType: Send + 'static, const CHANNEL_SIZE: usize = 100>:
    AsyncSinkWorker<T, WorkerReturnType> + Send + Sync + 'static
{
    async fn start(
        self,
    ) -> (
        JoinHandle<Result<WorkerReturnType>>,
        ThingbufSender<T>,
        oneshot::Sender<()>,
    )
    where
        Self: Sized,
        T: ChannelType,
    {
        let (thingbuf_tx, thingbuf_rx) = thingbuf::mpsc::channel::<T>(CHANNEL_SIZE);
        let (oneshot_tx, oneshot_rx) = oneshot::channel::<()>();

        let join_handle = tokio::spawn(self.run(thingbuf_rx, oneshot_rx));

        (join_handle, thingbuf_tx, oneshot_tx)
    }
}

#[async_trait::async_trait]
pub trait AsyncStreamWorker<T: ChannelType, ReturnType> {
    async fn run(self, tx: ThingbufSender<T>, rx: oneshot::Receiver<()>) -> Result<ReturnType>;
}

#[async_trait::async_trait]
pub trait Stream<T: ChannelType, WorkerReturnType: Send + 'static, const CHANNEL_SIZE: usize = 100>:
    AsyncStreamWorker<T, WorkerReturnType> + Send + Sync + 'static
{
    async fn start(
        self,
    ) -> (
        JoinHandle<Result<WorkerReturnType>>,
        ThingbufReceiver<T>,
        oneshot::Sender<()>,
    )
    where
        Self: Sized,
        T: ChannelType,
    {
        let (thingbuf_tx, thingbuf_rx) = thingbuf::mpsc::channel::<T>(CHANNEL_SIZE);
        let (oneshot_tx, oneshot_rx) = oneshot::channel::<()>();

        let join_handle = tokio::spawn(self.run(thingbuf_tx, oneshot_rx));

        (join_handle, thingbuf_rx, oneshot_tx)
    }
}

#[async_trait::async_trait]
pub trait AsyncLinkWorker<ConsumedType: ChannelType, ProducedType: ChannelType, ReturnType> {
    async fn run(
        self,
        rx: ThingbufReceiver<ConsumedType>,
        tx: ThingbufSender<ProducedType>,
        oneshot: oneshot::Receiver<()>,
    ) -> Result<ReturnType>;
}

#[async_trait::async_trait]
pub trait Link<
    ConsumedType: ChannelType,
    ProducedType: ChannelType,
    WorkerReturnType: Send + 'static,
    const SINK_CHANNEL_SIZE: usize = 100,
    const STREAM_CHANNEL_SIZE: usize = 100,
>: Send + AsyncLinkWorker<ConsumedType, ProducedType, WorkerReturnType> + 'static
{
    async fn start(
        self,
    ) -> (
        JoinHandle<Result<WorkerReturnType>>,
        ThingbufSender<ConsumedType>,
        ThingbufReceiver<ProducedType>,
        oneshot::Sender<()>,
    )
    where
        Self: Sized,
        ConsumedType: ChannelType,
        ProducedType: ChannelType,
    {
        let (inner_tx, inner_rx) = thingbuf::mpsc::channel::<ConsumedType>(SINK_CHANNEL_SIZE);
        let (outer_tx, outer_rx) = thingbuf::mpsc::channel::<ProducedType>(STREAM_CHANNEL_SIZE);
        let (oneshot_tx, oneshot_rx) = oneshot::channel::<()>();

        let join_handle = tokio::spawn(self.run(inner_rx, outer_tx, oneshot_rx));

        (join_handle, inner_tx, outer_rx, oneshot_tx)
    }
}

pub struct LinkWrapper<
    InputType: ChannelType,
    OutputType: ChannelType,
    CommunicationType: ChannelType,
    Link1: Link<InputType, CommunicationType, (), L1_SINK_CHANNEL_SIZE, L1_STREAM_CHANNEL_SIZE>,
    Link2: Link<CommunicationType, OutputType, (), L2_SINK_CHANNEL_SIZE, L2_STREAM_CHANNEL_SIZE>,
    const L1_SINK_CHANNEL_SIZE: usize = 100,
    const L1_STREAM_CHANNEL_SIZE: usize = 100,
    const L2_SINK_CHANNEL_SIZE: usize = 100,
    const L2_STREAM_CHANNEL_SIZE: usize = 100,
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
    async fn run(
        self,
        rx: ThingbufReceiver<InputType>,
        tx: ThingbufSender<OutputType>,
        oneshot: oneshot::Receiver<()>,
    ) -> Result<()> {
        let (link1_join_handle, link1_tx, link1_rx, lhs_oneshot_tx) = self.link1.start().await;
        let (link2_join_handle, link2_tx, link2_rx, rhs_oneshot_tx) = self.link2.start().await;

        use tokio::spawn;

        let branch_channels = spawn(branch_channels(
            oneshot,
            vec![lhs_oneshot_tx, rhs_oneshot_tx],
        ));

        let link_in = spawn(link_channels(rx, link1_tx));
        let link_out = spawn(link_channels(link2_rx, tx));

        let link_internal = spawn(link_channels(link1_rx, link2_tx));

        let _ = tokio::join!(
            link1_join_handle,
            link2_join_handle,
            branch_channels,
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

pub async fn link_channels<T: ChannelType>(
    rx: ThingbufReceiver<T>,
    tx: ThingbufSender<T>,
) -> Result<()> {
    while let Some(i) = rx.recv().await {
        tx.send(i).await?;
    }

    Ok(())
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
    ($oneshot_rx:expr, $stream:expr, $($link:expr);+, $sink:expr) => {{
        async move {
            let stream = $stream;
            let link = eval_links!($($link),+);
            let sink = $sink;

            use tokio::spawn;

            let mut branches = Vec::new();

            let (stream_join_handle, stream_rx, oneshot_tx) = stream.start().await;
            branches.push(oneshot_tx);

            let (link_join_handle, link_tx, link_rx, oneshot_tx) = link.start().await;
            branches.push(oneshot_tx);

            let (sink_join_handle, sink_tx, oneshot_tx) = sink.start().await; //TODO oneshot
            branches.push(oneshot_tx);

            let link1 = spawn(link_channels(stream_rx, link_tx));
            let link2 = spawn(link_channels(link_rx, sink_tx));


            let oneshot_rx = $oneshot_rx;
            let branch_channels = spawn(branch_channels(oneshot_rx, branches));

            let _ = tokio::join!(
                stream_join_handle,
                link_join_handle,
                sink_join_handle,
                link1,
                link2,
                branch_channels
            );

            //TODO return result (or log error!)
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SimpleStream {}

    #[async_trait::async_trait]
    impl AsyncStreamWorker<u32, ()> for SimpleStream {
        async fn run(
            self,
            tx: ThingbufSender<u32>,
            _oneshot_rx: oneshot::Receiver<()>,
        ) -> Result<()> {
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
        async fn run(
            self,
            rx: ThingbufReceiver<u32>,
            tx: ThingbufSender<u32>,
            _oneshot_rx: oneshot::Receiver<()>,
        ) -> Result<()> {
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
        async fn run(
            self,
            rx: ThingbufReceiver<u32>,
            tx: ThingbufSender<u32>,
            _oneshot_rx: oneshot::Receiver<()>,
        ) -> Result<()> {
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
        async fn run(
            self,
            rx: ThingbufReceiver<u32>,
            tx: ThingbufSender<u32>,
            _oneshot_rx: oneshot::Receiver<()>,
        ) -> Result<()> {
            while let Some(i) = rx.recv().await {
                tx.send(i / 2).await?;
            }

            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Link<u32, u32, ()> for Div2 {}

    struct SimpleSink {
        out_tx: ThingbufSender<u32>, //used outside of pipeline
    }

    #[async_trait::async_trait]
    impl AsyncSinkWorker<u32, ()> for SimpleSink {
        async fn run(
            self,
            rx: ThingbufReceiver<u32>,
            _oneshot_rx: oneshot::Receiver<()>,
        ) -> Result<()> {
            while let Some(i) = rx.recv().await {
                self.out_tx.send(i).await?;
            }

            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Sink<u32, ()> for SimpleSink {}

    #[tokio::test]
    async fn test_macro() {
        use tokio::spawn;

        let stream = SimpleStream {};
        let x3 = X3 {};
        let x10 = X10 {};
        let div2 = Div2 {};

        let link = eval_links!(x3, x10, div2);

        let (pipeline_tx, pipeline_rx) = thingbuf::mpsc::channel::<u32>(10);
        let sink = SimpleSink {
            out_tx: pipeline_tx,
        };

        let (oneshot_tx, oneshot_rx) = oneshot::channel();

        let pipeline_task = spawn(start_and_link_all!(oneshot_rx, stream, link, sink));

        let receiver_task = spawn(async move {
            let mut i = 0;
            while let Some(j) = pipeline_rx.recv().await {
                assert_eq!(j, i * 15);
                i += 1;
            }
        });

        let sleep_task = spawn(tokio::time::sleep(tokio::time::Duration::from_millis(100)));

        match tokio::select! {
            _ = receiver_task => { Ok(()) },
            _ = sleep_task => { Err(()) },
        } {
            Ok(_) => {}
            Err(_) => {
                panic!("receiver task failed");
            }
        }

        oneshot_tx.send(()).unwrap();

        let sleep_task = spawn(tokio::time::sleep(tokio::time::Duration::from_millis(100)));

        match tokio::select! {
            _ = pipeline_task => { Ok(()) },
            _ = sleep_task => { Err(()) },
        } {
            Ok(_) => {}
            Err(_) => {
                panic!("pipeline task did not finish!");
            }
        }
    }
}
