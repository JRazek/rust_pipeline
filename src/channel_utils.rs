use super::sink_stream::Result;
use tokio::sync::oneshot;

pub trait ChannelType: Send + Sync + Clone + Default + 'static {}

pub async fn branch_oneshot_channels<T: ChannelType + std::fmt::Debug>(
    input_rx: oneshot::Receiver<T>,
    branches: Vec<oneshot::Sender<T>>,
) -> Result<()> {
    let input = input_rx.await?;

    let matcher = |x| match x {
        Ok(_) => Ok(()),
        Err(_) => {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Error sending to channel").into())
        }
    };

    let _branches = branches
        .into_iter()
        .map(|tx| matcher(tx.send(input.clone())))
        .collect::<Result<()>>();
    //TODO return error

    Ok(())
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
