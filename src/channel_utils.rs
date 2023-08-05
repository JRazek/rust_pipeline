use super::sink_stream::Result;
use futures::StreamExt;
use tokio::sync::oneshot;

pub trait ChannelType: Send + Sync + Clone + Default + 'static {}

pub async fn branch_oneshot_channels<T: ChannelType + std::fmt::Debug>(
    input_rx: oneshot::Receiver<T>,
    branches: Vec<oneshot::Sender<T>>,
) -> Result<()> {
    let input = input_rx.await?;

    eprintln!("branch_oneshot_channel input: {:?}", input);

    let branches = branches.into_iter().for_each(|tx| {
        if !tx.is_closed() {
            match tx.send(input.clone()) {
                Ok(_) => {}
                Err(_) => {
                    eprintln!("Error sending to channel in branch_oneshot_channel");
                }
            }
        }
    });

    eprintln!("branch_oneshot_channel results: {:?}", branches);

    Ok(())
}

pub async fn join_oneshot_channels<T: ChannelType + std::fmt::Debug>(
    inputs_rx: Vec<oneshot::Receiver<T>>,
    output_tx: oneshot::Sender<T>,
) -> Result<()> {
    use futures::stream::FuturesUnordered;

    let mut tasks = inputs_rx.into_iter().collect::<FuturesUnordered<_>>();

    let res = match tasks.next().await {
        Some(Ok(res)) => output_tx.send(res).map_err(|_| {
            eprintln!("Error sending to channel");
            std::io::Error::new(std::io::ErrorKind::Other, "Error sending to channel").into()
        }),
        Some(Err(_)) => {
            eprintln!("Error receiving from channel");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Error").into());
        }
        _ => {
            eprintln!("Error receiving from channel");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Error").into());
        }
    };

    eprintln!("join_oneshot_channels: {:?}", res);

    res
}

use thingbuf::mpsc::{Receiver as ThingbufReceiver, Sender as ThingbufSender};

pub async fn link_thingbuf_channels<T: ChannelType>(
    rx: ThingbufReceiver<T>,
    tx: ThingbufSender<T>,
) -> Result<()> {
    while let Some(i) = rx.recv().await {
        tx.send(i).await?;
    }

    Ok(())
}

pub async fn link_oneshot_channels<T: Send + Sync>(
    rx: oneshot::Receiver<T>,
    tx: oneshot::Sender<T>,
) -> Result<()> {
    if let Ok(i) = rx.await {
        if let Err(_) = tx.send(i) {
            return Err(
                std::io::Error::new(std::io::ErrorKind::Other, "could not pipe oneshot!").into(),
            );
        }
    }

    Ok(())
}
