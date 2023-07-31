pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[async_trait::async_trait]
pub trait Stream<T: Send> {
    async fn produce<SinkType: Sink<T> + Send>(&mut self) -> Result<T>;

    async fn send<SinkType: Sink<T> + Send>(&mut self, sink: &mut SinkType) -> Result<()> {
        let data = self.produce::<SinkType>().await?;
        sink.consume(data).await?;

        Ok(())
    }
}

#[async_trait::async_trait]
pub trait Sink<T> {
    async fn consume(&mut self, data: T) -> Result<()>;
}

#[async_trait::async_trait]
pub trait Link<ConsumedType, ProducedType: Send>:
    Sink<ConsumedType> + Stream<ProducedType>
{
    async fn link_and_spawn_worker<LinkSinkType: Sink<ProducedType> + Send + 'static>(
        &'static mut self,
        mut sink: LinkSinkType,
        //        shutdown: tokio::sync::oneshot::Receiver<()>,
    ) {
        tokio::spawn(async move {
            loop {
                let data = self.produce::<LinkSinkType>().await.unwrap();
                sink.consume(data).await.unwrap();
            }
        });
    }
}

#[cfg(test)]
mod test {
    use thingbuf::mpsc::{Receiver, Sender};

    use super::Sink;

    struct SimpleStream {}

    struct SimpleLink {
        queue: std::collections::VecDeque<u32>,
        out_rx: Receiver<u32>,
        out_tx: Sender<u32>,
    }

    impl SimpleLink {
        async fn process(&mut self) {
            if let Some(mut data) = self.queue.pop_front() {
                data *= 2;
                self.out_tx.send(data).await.unwrap();
            }
        }
    }

    struct SimpleSink {
        pub data: u32,
    }

    #[async_trait::async_trait]
    impl super::Stream<u32> for SimpleStream {
        async fn produce<SinkType: Sink<u32> + Send>(&mut self) -> super::Result<u32> {
            Ok(5)
        }
    }

    impl SimpleLink {
        pub fn new() -> Self {
            let (tx, rx) = thingbuf::mpsc::channel(100);
            Self {
                queue: std::collections::VecDeque::new(),
                out_rx: rx,
                out_tx: tx,
            }
        }
    }

    #[async_trait::async_trait]
    impl super::Sink<u32> for SimpleLink {
        async fn consume(&mut self, data: u32) -> super::Result<()> {
            self.queue.push_back(data);
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl super::Stream<u32> for SimpleLink {
        async fn produce<SinkType: Sink<u32> + Send>(&mut self) -> super::Result<u32> {
            self.process().await;
            let data = self.out_rx.recv().await.unwrap();

            Ok(data)
        }
    }

    #[async_trait::async_trait]
    impl super::Sink<u32> for SimpleSink {
        async fn consume(&mut self, data: u32) -> super::Result<()> {
            self.data = data;
            Ok(())
        }
    }

    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn test_link_all() {
        let mut stream = Arc::new(Mutex::new(SimpleStream {}));
        let mut link = Arc::new(Mutex::new(SimpleLink::new()));
        let mut sink = Arc::new(Mutex::new(SimpleSink { data: 0 }));

        use super::{Link, Sink, Stream};

        //        let _linked = link_all!(stream, &mut link, &sink);
    }
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");
}
