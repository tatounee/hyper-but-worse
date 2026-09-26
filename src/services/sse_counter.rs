use std::{
    hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hash, Hasher},
    task::{Poll, ready},
    time::Duration,
};

use futures::Stream;
use pin_project_lite::pin_project;
use tokio::time::{Interval, interval};

use serde::Serialize;

pin_project! {
    pub struct CounterStream {
        #[pin]
        ticker: Interval,
        value: usize,
    }
}

impl Default for CounterStream {
    fn default() -> Self {
        Self {
            ticker: interval(Duration::from_secs(1)),
            value: 0,
        }
    }
}

impl Clone for CounterStream {
    fn clone(&self) -> Self {
        Self::default()
    }
}

#[derive(Serialize)]
pub struct CounterValue {
    value: usize,
    text: String,
    hash: u64,
}

impl CounterValue {
    fn new(value: usize) -> Self {
        let text = value.to_string();

        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        let hash = hasher.finish();

        Self { value, text, hash }
    }
}

impl Stream for CounterStream {
    type Item = CounterValue;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        ready!(this.ticker.poll_tick(cx));

        let data = CounterValue::new(*this.value);
        *this.value += 1;

        Poll::Ready(Some(data))
    }
}
