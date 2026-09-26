use std::task::Poll;

use bytes::{Bytes, BytesMut};
use futures::Stream;
use pin_project_lite::pin_project;

use super::SSEvent;

pin_project! {
    #[derive(Debug, Clone)]
    pub(super) struct SSEventToBytesStream<S> {
        #[pin]
        stream: S,
    }
}

impl<S> SSEventToBytesStream<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
}

impl<S, T> Stream for SSEventToBytesStream<S>
where
    S: Stream<Item = T>,
    T: SSEvent,
{
    type Item = Bytes;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();

        this.stream.poll_next(cx).map(|event| {
            event.map(|event| {
                let bytes = event.into_bytes();

                let mut buf = BytesMut::from(bytes);
                buf.extend_from_slice(b"\n");
                buf.freeze()
            })
        })
    }
}
