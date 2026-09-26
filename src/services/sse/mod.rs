use std::io::{BufWriter, Write};
use std::mem;
use std::task::Poll;

use bytes::Bytes;
use bytes_stream::SSEventToBytesStream;
use color_eyre::Report;
use futures::{Stream, StreamExt};
use http::Response;
use serde::Serialize;
use serde_json::ser::{CompactFormatter, Serializer as JsonSerializer};
use tower::Service;

mod bytes_stream;
// mod serializer;

use crate::body::Body;

#[derive(Clone)]
pub struct ServerSendEvent<T> {
    stream: T,
}

impl<T> ServerSendEvent<T> {
    pub fn new(stream: T) -> Self {
        Self { stream }
    }
}

impl<T, U, R> Service<R> for ServerSendEvent<T>
where
    T: Stream<Item = U> + Send + Clone + 'static,
    U: SSEvent,
{
    type Response = Response<Body>;

    type Error = Report;

    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: R) -> Self::Future {
        let cloned_stream = self.stream.clone();
        let stream = mem::replace(&mut self.stream, cloned_stream);

        let body = Body::Stream(SSEventToBytesStream::new(stream).boxed());
        let response = Response::builder()
            .status(200)
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("X-Accel-Buffering", "no")
            .body(body)
            .map_err(Report::new);

        async move { response }
    }
}

pub trait SSEvent {
    fn into_bytes(self) -> Bytes;
}

impl<T> SSEvent for T
where
    T: Serialize,
{
    fn into_bytes(self) -> Bytes {
        let mut buf = BufWriter::new(Vec::new());

        buf.write_all(b"data:").unwrap();

        let mut serializer = JsonSerializer::with_formatter(buf, CompactFormatter);
        // This unwrap can panic :(
        self.serialize(&mut serializer).unwrap();

        let mut buf = serializer.into_inner();
        buf.write_all(b"\n").unwrap();

        let buf = buf.into_inner().unwrap();

        Bytes::from(buf)
    }
}

// -------------------------------------------------------------------------- //

#[allow(dead_code)]
pub struct FullSSEvent<T> {
    data: T,
    event: Option<&'static str>,
}

#[allow(dead_code)]
impl<T> FullSSEvent<T>
where
    T: Serialize,
{
    pub fn new(data: T) -> Self {
        Self { data, event: None }
    }

    pub fn with_event(data: T, event: &'static str) -> Self {
        Self {
            data,
            event: Some(event),
        }
    }
}

impl<T> SSEvent for FullSSEvent<T>
where
    T: Serialize,
{
    fn into_bytes(self) -> Bytes {
        let mut buf = BufWriter::new(Vec::new());

        if let Some(event) = self.event {
            buf.write_all(event.as_bytes()).unwrap();
            buf.write_all(b"\n").unwrap();
        }

        buf.write_all(b"data:").unwrap();

        let mut serializer = JsonSerializer::with_formatter(buf, CompactFormatter);
        // This unwrap can panic :(
        self.data.serialize(&mut serializer).unwrap();

        let mut buf = serializer.into_inner();
        buf.write_all(b"\n").unwrap();

        let buf = buf.into_inner().unwrap();

        Bytes::from(buf)
    }
}
