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
    ($deadline_oneshot_rx:expr, $stream:expr, $($link:expr);+, $sink:expr) => {{
        async move {
            use futures::stream::FuturesUnordered;
            use futures::stream::StreamExt;
            use crate::channel_utils::branch_oneshot_channels;
            use crate::channel_utils::link_thingbuf_channels;
            use crate::sink_stream::*;
            use tokio::sync::oneshot;

            let stream = $stream;
            let link = eval_links!($($link),+);
            let sink = $sink;

            use tokio::spawn;

            async fn await_oneshot(oneshot: oneshot::Receiver<()>) -> Result<()> {
                oneshot.await?;
                Ok(())
            }

            let mut branches = Vec::new();

            let (stream_join_handle, stream_rx, oneshot_tx) = stream.start().await;
            branches.push(oneshot_tx);

            let (link_join_handle, link_tx, link_rx, oneshot_tx) = link.start().await;
            branches.push(oneshot_tx);

            let (sink_join_handle, sink_tx, oneshot_tx) = sink.start().await;
            branches.push(oneshot_tx);

            let stream_link = spawn(link_thingbuf_channels(stream_rx, link_tx));
            let link_sink = spawn(link_thingbuf_channels(link_rx, sink_tx));

            let (oneshot_tx, oneshot_rx) = oneshot::channel::<()>();

            let branch_channels = spawn(branch_oneshot_channels(oneshot_rx, branches));

            let mut futures = FuturesUnordered::new();

            let deadline_oneshot_task = spawn(await_oneshot($deadline_oneshot_rx));

            futures.push(deadline_oneshot_task);
            futures.push(stream_join_handle);
            futures.push(link_join_handle);
            futures.push(sink_join_handle);
            futures.push(stream_link);
            futures.push(link_sink);
            futures.push(branch_channels);

            if let Some(res) = futures.next().await {
                match res {
                    Ok(Ok(_)) => {
                        oneshot_tx.send(()).expect("Failed to send oneshot");
                    },
                    Ok(Err(e)) => {
                        return Err(e);
                    },
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }

            while let Some(res) = futures.next().await {
                match res {
                    Err(e) => {
                        return Err(e.into());
                    }
                    Ok(Err(_)) => {
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Error").into());
                    }
                    Ok(Ok(_)) => {}
                }
            }

            Ok(())
        }
    }};
}
