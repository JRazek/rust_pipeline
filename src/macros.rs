#[macro_export]
macro_rules! eval_links {
    ($link:expr) => {
        $link
    };

    ($link:expr, $($links:expr),+) => {{
        use $crate::sink_stream::*;
        LinkWrapper::new($link, eval_links!($($links),*))
    }};
}

#[macro_export]
macro_rules! start_and_link_all {
    ($deadline_oneshot_rx:expr, $stream:expr, $($link:expr);+, $sink:expr) => {{
        async move {
            use futures::stream::FuturesUnordered;
            use futures::stream::StreamExt;

            use $crate::channel_utils::branch_oneshot_channels;
            use $crate::channel_utils::link_thingbuf_channels;
            use $crate::channel_utils::join_oneshot_channels;
            use $crate::sink_stream::*;

            use tokio::sync::oneshot;

            let stream = $stream;
            let link = eval_links!($($link),+);
            let sink = $sink;
            let deadline_oneshot_rx = $deadline_oneshot_rx;

            use tokio::spawn;

            let mut outer_joints = Vec::new();
            let mut inner_branches = Vec::new();

            outer_joints.push(deadline_oneshot_rx);

            let (stream_join_handle, stream_rx, oneshot_tx) = stream.start().await;
            inner_branches.push(oneshot_tx);

            let (link_join_handle, link_tx, link_rx, oneshot_tx) = link.start().await;
            inner_branches.push(oneshot_tx);

            let (sink_join_handle, sink_tx, oneshot_tx) = sink.start().await;
            inner_branches.push(oneshot_tx);

            let stream_link = spawn(link_thingbuf_channels(stream_rx, link_tx));
            let link_sink = spawn(link_thingbuf_channels(link_rx, sink_tx));

            let (link_oneshot_tx, link_oneshot_rx) = oneshot::channel::<()>();

            let (feedback_loop_onshot_tx, feedback_loop_oneshot_rx) = oneshot::channel::<()>();
            outer_joints.push(feedback_loop_oneshot_rx);

            let joint = spawn(join_oneshot_channels(outer_joints, link_oneshot_tx));
            let branch_channels = spawn(branch_oneshot_channels(link_oneshot_rx, inner_branches));

            let mut futures = FuturesUnordered::new();

            futures.push(stream_join_handle);
            futures.push(link_join_handle);
            futures.push(sink_join_handle);
            futures.push(stream_link);
            futures.push(link_sink);
            futures.push(joint);
            futures.push(branch_channels);

            let pipeline_tasks = async move {
                if let Some(res) = futures.next().await {
                    match res {
                        Ok(Ok(_)) => {
                            eprintln!("feedback loop oneshot fired");
                            feedback_loop_onshot_tx.send(()).unwrap();
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
            };

            pipeline_tasks.await
        }
    }};
}
