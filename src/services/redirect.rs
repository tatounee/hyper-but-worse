use std::task::Poll;

use bytes::Bytes;
use color_eyre::Report;
use http::{Response, StatusCode};
use tower::Service;

use crate::body::Body;

#[derive(Debug, Clone)]
pub struct Redirect(String);

impl Redirect {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new<T: ToString>(location: T) -> Self {
        Self(location.to_string())
    }
}

impl<T> Service<T> for Redirect {
    type Response = Response<Body>;

    type Error = Report;

    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: T) -> Self::Future {
        let redirection = self.0.clone();

        async move {
            Response::builder()
                .status(StatusCode::TEMPORARY_REDIRECT)
                .header("location", redirection)
                .body(Body::Static(Bytes::new()))
                .map_err(Report::new)
        }
    }
}
