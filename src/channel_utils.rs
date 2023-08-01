use super::sink_stream::Result;
use tokio::sync::oneshot;

pub trait ChannelType: Send + Sync + Clone + Default + 'static {}

#[allow(dead_code)]
pub async fn branch_channels<T: ChannelType + std::fmt::Debug>(
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
