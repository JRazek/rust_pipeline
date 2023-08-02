use super::channel_utils::{branch_oneshot_channels, ChannelType};
use std::marker::PhantomData;
use tokio::{sync::oneshot, task::JoinHandle};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

use thingbuf::mpsc::{Receiver as ThingbufReceiver, Sender as ThingbufSender};

use super::channel_utils::link_thingbuf_channels;

//TODO when stable rust: trait ChannelType = Send + Sync + Default + 'static;
impl<T: Send + Sync + Clone + Default + 'static> ChannelType for T {}

#[async_trait::async_trait]
pub trait AsyncSinkWorker<T: ChannelType, UserData> {
    async fn run(
        user_data: UserData,
        rx: ThingbufReceiver<T>,
        oneshot: oneshot::Receiver<()>,
    ) -> Result<()>;
}

#[async_trait::async_trait]
pub trait Sink<T: ChannelType, const CHANNEL_SIZE: usize = 100>: Sized {
    type AsyncSinkWorkerType: AsyncSinkWorker<T, Self> + Send + Sync + 'static;

    async fn start(
        self,
    ) -> (
        JoinHandle<Result<()>>,
        ThingbufSender<T>,
        oneshot::Sender<()>,
    )
    where
        Self: Sized,
        T: ChannelType,
    {
        let (thingbuf_tx, thingbuf_rx) = thingbuf::mpsc::channel::<T>(CHANNEL_SIZE);
        let (oneshot_tx, oneshot_rx) = oneshot::channel::<()>();

        let join_handle = tokio::spawn(Self::AsyncSinkWorkerType::run(
            self,
            thingbuf_rx,
            oneshot_rx,
        ));

        (join_handle, thingbuf_tx, oneshot_tx)
    }
}

#[async_trait::async_trait]
pub trait AsyncStreamWorker<T: ChannelType, UserData> {
    async fn run(
        user_data: UserData,
        tx: ThingbufSender<T>,
        rx: oneshot::Receiver<()>,
    ) -> Result<()>;
}

#[async_trait::async_trait]
pub trait Stream<T: ChannelType, const CHANNEL_SIZE: usize = 100>: Sized {
    type AsyncStreamWorkerType: AsyncStreamWorker<T, Self>;

    async fn start(
        self,
    ) -> (
        JoinHandle<Result<()>>,
        ThingbufReceiver<T>,
        oneshot::Sender<()>,
    )
    where
        Self: Sized,
        T: ChannelType,
    {
        let (thingbuf_tx, thingbuf_rx) = thingbuf::mpsc::channel::<T>(CHANNEL_SIZE);
        let (oneshot_tx, oneshot_rx) = oneshot::channel::<()>();

        let join_handle = tokio::spawn(Self::AsyncStreamWorkerType::run(
            self,
            thingbuf_tx,
            oneshot_rx,
        ));

        (join_handle, thingbuf_rx, oneshot_tx)
    }
}

#[async_trait::async_trait]
pub trait AsyncLinkWorker<ConsumedType: ChannelType, ProducedType: ChannelType, UserData> {
    async fn run(
        user_data: UserData,
        rx: ThingbufReceiver<ConsumedType>,
        tx: ThingbufSender<ProducedType>,
        oneshot: oneshot::Receiver<()>,
    ) -> Result<()>;
}

#[async_trait::async_trait]
pub trait Link<
    ConsumedType: ChannelType,
    ProducedType: ChannelType,
    const SINK_CHANNEL_SIZE: usize = 100,
    const STREAM_CHANNEL_SIZE: usize = 100,
>: Send + Sized
{
    type AsyncLinkWorkerType: AsyncLinkWorker<ConsumedType, ProducedType, Self>;

    async fn start(
        self,
    ) -> (
        JoinHandle<Result<()>>,
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

        let join_handle = tokio::spawn(Self::AsyncLinkWorkerType::run(
            self, inner_rx, outer_tx, oneshot_rx,
        ));

        (join_handle, inner_tx, outer_rx, oneshot_tx)
    }
}

pub struct LinkWrapper<
    InputType: ChannelType,
    OutputType: ChannelType,
    CommunicationType: ChannelType,
    Link1: Link<InputType, CommunicationType, L1_SINK_CHANNEL_SIZE, L1_STREAM_CHANNEL_SIZE>,
    Link2: Link<CommunicationType, OutputType, L2_SINK_CHANNEL_SIZE, L2_STREAM_CHANNEL_SIZE>,
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
        Link1: Link<InputType, CommunicationType>,
        Link2: Link<CommunicationType, OutputType>,
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

pub struct LinkWrapperWorker {}

#[async_trait::async_trait]
impl<
        InputType: ChannelType,
        OutputType: ChannelType,
        CommunicationType: ChannelType,
        Link1: Link<InputType, CommunicationType> + Send + 'static,
        Link2: Link<CommunicationType, OutputType> + Send + 'static,
    >
    AsyncLinkWorker<
        InputType,
        OutputType,
        LinkWrapper<InputType, OutputType, CommunicationType, Link1, Link2>,
    > for LinkWrapperWorker
{
    async fn run(
        user_data: LinkWrapper<InputType, OutputType, CommunicationType, Link1, Link2>,
        rx: ThingbufReceiver<InputType>,
        tx: ThingbufSender<OutputType>,
        oneshot: oneshot::Receiver<()>,
    ) -> Result<()> {
        let (link1_join_handle, link1_tx, link1_rx, lhs_oneshot_tx) = user_data.link1.start().await;
        let (link2_join_handle, link2_tx, link2_rx, rhs_oneshot_tx) = user_data.link2.start().await;

        use tokio::spawn;

        let branch_channels = spawn(branch_oneshot_channels(
            oneshot,
            vec![lhs_oneshot_tx, rhs_oneshot_tx],
        ));

        let link_in = spawn(link_thingbuf_channels(rx, link1_tx));
        let link_out = spawn(link_thingbuf_channels(link2_rx, tx));

        let link_internal = spawn(link_thingbuf_channels(link1_rx, link2_tx));

        let res = tokio::try_join!(
            link1_join_handle,
            link2_join_handle,
            branch_channels,
            link_in,
            link_out,
            link_internal
        );

        match res {
            Ok((Ok(_), Ok(_), Ok(_), Ok(_), Ok(_), Ok(_))) => Ok(()),
            _ => Err(
                std::io::Error::new(std::io::ErrorKind::Other, "LinkWrapperWorker failed").into(),
            ),
        }
    }
}

#[async_trait::async_trait]
impl<
        InputType: ChannelType,
        OutputType: ChannelType,
        CommunicationType: ChannelType,
        Link1: Link<InputType, CommunicationType> + Send + 'static,
        Link2: Link<CommunicationType, OutputType> + Send + 'static,
    > Link<InputType, OutputType>
    for LinkWrapper<InputType, OutputType, CommunicationType, Link1, Link2>
{
    type AsyncLinkWorkerType = LinkWrapperWorker;
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

            let (sink_join_handle, sink_tx, oneshot_tx) = sink.start().await;
            branches.push(oneshot_tx);

            let stream_link = spawn(link_thingbuf_channels(stream_rx, link_tx));
            let link_sink = spawn(link_thingbuf_channels(link_rx, sink_tx));

            let oneshot_rx = $oneshot_rx;
            let branch_channels = spawn(branch_oneshot_channels(oneshot_rx, branches));

            let res = tokio::try_join!(
                stream_join_handle,
                link_join_handle,
                sink_join_handle,
                stream_link,
                link_sink,
                branch_channels
            );

            match res {
                Ok((Ok(_), Ok(_), Ok(_), Ok(_), Ok(_), Ok(_))) => Ok(()),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Error in start_and_link_all",
                )),
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SimpleStream {}

    struct SimpleAsyncStreamWorker {}

    #[async_trait::async_trait]
    impl AsyncStreamWorker<u32, SimpleStream> for SimpleAsyncStreamWorker {
        async fn run(
            _user_data: SimpleStream,
            tx: ThingbufSender<u32>,
            oneshot_rx: oneshot::Receiver<()>,
        ) -> Result<()> {
            let oneshot_task = tokio::spawn(async move { oneshot_rx.await });

            let loop_task = tokio::spawn(async move {
                for i in 0..10 {
                    tx.send(i).await?;
                }

                Ok::<(), thingbuf::mpsc::errors::Closed<_>>(())
            });

            let res = tokio::select!(
                res = oneshot_task => {
                    match res {
                        Ok(_) => Ok(()),
                        Err(_) => Err(std::io::Error::new(std::io::ErrorKind::Other, "oneshot error").into())
                    }
                },
                res = loop_task => {
                    match res {
                        Ok(_) => Ok(()),
                        Err(_) => Err(std::io::Error::new(std::io::ErrorKind::Other, "loop error").into())
                    }
                }
            );

            res
        }
    }

    #[async_trait::async_trait]
    impl Stream<u32> for SimpleStream {
        type AsyncStreamWorkerType = SimpleAsyncStreamWorker;
    }

    struct X3 {}
    struct X3Worker {}

    #[async_trait::async_trait]
    impl AsyncLinkWorker<u32, u32, X3> for X3Worker {
        async fn run(
            _: X3,
            rx: ThingbufReceiver<u32>,
            tx: ThingbufSender<u32>,
            oneshot_rx: oneshot::Receiver<()>,
        ) -> Result<()> {
            let oneshot_task = tokio::spawn(async move { oneshot_rx.await });

            let loop_task = tokio::spawn(async move {
                while let Some(i) = rx.recv().await {
                    tx.send(i * 3).await?;
                }

                Ok::<(), thingbuf::mpsc::errors::Closed<_>>(())
            });

            let res = tokio::select!(
                res = oneshot_task => {
                    match res {
                        Ok(_) => Ok(()),
                        Err(_) => Err(std::io::Error::new(std::io::ErrorKind::Other, "oneshot error").into())
                    }
                },
                res = loop_task => {
                    match res {
                        Ok(_) => Ok(()),
                        Err(_) => Err(std::io::Error::new(std::io::ErrorKind::Other, "loop error").into())
                    }
                }
            );

            res
        }
    }

    #[async_trait::async_trait]
    impl Link<u32, u32> for X3 {
        type AsyncLinkWorkerType = X3Worker;
    }

    struct X10 {}
    struct X10Worker {}

    #[async_trait::async_trait]
    impl AsyncLinkWorker<u32, u32, X10> for X10Worker {
        async fn run(
            _: X10,
            rx: ThingbufReceiver<u32>,
            tx: ThingbufSender<u32>,
            oneshot_rx: oneshot::Receiver<()>,
        ) -> Result<()> {
            let oneshot_task = tokio::spawn(async move { oneshot_rx.await });

            let loop_task = tokio::spawn(async move {
                while let Some(i) = rx.recv().await {
                    tx.send(i * 10).await?;
                }

                Ok::<(), thingbuf::mpsc::errors::Closed<_>>(())
            });

            let res = tokio::select!(
                res = oneshot_task => {
                    match res {
                        Ok(_) => Ok(()),
                        _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Error in Div2Worker").into()),
                    }
                },
                res = loop_task => {
                    match res {
                        Ok(Ok(_)) => Ok(()),
                        _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Error in Div2Worker").into()),
                    }
                }
            );

            res
        }
    }

    #[async_trait::async_trait]
    impl Link<u32, u32> for X10 {
        type AsyncLinkWorkerType = X10Worker;
    }

    struct Div2 {}
    struct Div2Worker {}

    #[async_trait::async_trait]
    impl AsyncLinkWorker<u32, u32, Div2> for Div2Worker {
        async fn run(
            _: Div2,
            rx: ThingbufReceiver<u32>,
            tx: ThingbufSender<u32>,
            oneshot_rx: oneshot::Receiver<()>,
        ) -> Result<()> {
            let oneshot_task = tokio::spawn(async move { oneshot_rx.await });

            let loop_task = tokio::spawn(async move {
                while let Some(i) = rx.recv().await {
                    tx.send(i / 2).await?;
                }

                Ok::<(), thingbuf::mpsc::errors::Closed<_>>(())
            });

            let res = tokio::select!(
                res = oneshot_task => {
                    match res {
                        Ok(_) => Ok(()),
                        _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Error in Div2Worker").into()),
                    }
                },
                res = loop_task => {
                    match res {
                        Ok(Ok(_)) => Ok(()),
                        _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Error in Div2Worker").into()),
                    }
                }
            );

            res
        }
    }

    #[async_trait::async_trait]
    impl Link<u32, u32> for Div2 {
        type AsyncLinkWorkerType = Div2Worker;
    }

    struct SimpleSink {
        out_tx: ThingbufSender<u32>,
    }

    struct SimpleAsyncSinkWorker {}

    #[async_trait::async_trait]
    impl AsyncSinkWorker<u32, SimpleSink> for SimpleAsyncSinkWorker {
        async fn run(
            user_data: SimpleSink,
            rx: ThingbufReceiver<u32>,
            oneshot_rx: oneshot::Receiver<()>,
        ) -> Result<()> {
            let oneshot_task = tokio::spawn(async move {
                let res = oneshot_rx.await;

                println!("SimpleSink oneshot_task done");

                res
            });

            let loop_task = tokio::spawn(async move {
                while let Some(i) = rx.recv().await {
                    user_data.out_tx.send(i).await?;
                }

                Ok::<(), thingbuf::mpsc::errors::Closed<_>>(())
            });

            let res = tokio::select!(
                res = oneshot_task => {
                    match res {
                        Ok(_) => Ok(()),
                        _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Error in Div2Worker").into()),
                    }
                },
                res = loop_task => {
                    match res {
                        Ok(Ok(_)) => Ok(()),
                        _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Error in Div2Worker").into()),
                    }
                }
            );

            res
        }
    }

    #[async_trait::async_trait]
    impl Sink<u32> for SimpleSink {
        type AsyncSinkWorkerType = SimpleAsyncSinkWorker;
    }

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

            println!("receiver task finished");

            assert_eq!(i, 10);
        });

        let sleep_task = spawn(tokio::time::sleep(tokio::time::Duration::from_millis(100)));

        match tokio::select! {
            Ok(_) = receiver_task => { Ok(()) },
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
            Ok(Ok(_)) = pipeline_task => { Ok(()) },
            _ = sleep_task => { Err(()) },
        } {
            Ok(_) => {}
            Err(_) => {
                panic!("pipeline task did not finish!");
            }
        }
    }

    struct FailingSink {}

    pub struct FailingAsyncSinkWorker {}

    #[async_trait::async_trait]
    impl Sink<u32> for FailingSink {
        type AsyncSinkWorkerType = FailingAsyncSinkWorker;
    }

    #[async_trait::async_trait]
    impl AsyncSinkWorker<u32, FailingSink> for FailingAsyncSinkWorker {
        async fn run(
            _user_data: FailingSink,
            _rx: ThingbufReceiver<u32>,
            _oneshot_rx: oneshot::Receiver<()>,
        ) -> Result<()> {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "FailingAsyncSinkWorker").into())
        }
    }

    #[tokio::test]
    async fn failing_sink() {
        use tokio::spawn;

        let stream = SimpleStream {};
        let x3 = X3 {};
        let x10 = X10 {};
        let div2 = Div2 {};

        let link = eval_links!(x3, x10, div2);

        let (_pipeline_tx, _pipeline_rx) = thingbuf::mpsc::channel::<u32>(10);
        let sink = FailingSink {};

        let (oneshot_tx, oneshot_rx) = oneshot::channel();

        let pipeline_task = spawn(start_and_link_all!(oneshot_rx, stream, link, sink));

        oneshot_tx.send(()).unwrap();

        let sleep_task = spawn(tokio::time::sleep(tokio::time::Duration::from_millis(100)));

        match tokio::select! {
            Ok(Err(_)) = pipeline_task => { Ok(()) },
            _ = sleep_task => { Err(()) },
        } {
            Ok(_) => {}
            Err(_) => {
                panic!("pipeline task did not finish with error!");
            }
        }
    }
}
