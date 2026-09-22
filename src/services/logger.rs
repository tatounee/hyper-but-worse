use std::mem;

use http::{Request, Response};
use tower::{Layer, Service};
use tracing::{debug, error, warn};

pub struct Logger<S> {
    inner: S,
}

impl<S, B, BB> Service<Request<B>> for Logger<S>
where
    S: Service<Request<B>, Response = Response<BB>> + Clone,
{
    type Response = Response<BB>;

    type Error = S::Error;

    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let method = req.method();
        let uri = req.uri().clone();

        debug!("{:4} {}", method.as_str(), uri);

        let cloned_service = self.inner.clone();
        let mut service = mem::replace(&mut self.inner, cloned_service);

        async move {
            let response = service.call(req).await?;

            if response.status().is_client_error() {
                warn!("{} {}", response.status(), uri);
            } else if response.status().is_server_error() {
                error!("{} {}", response.status(), uri);
            }

            Ok(response)
        }
    }
}

pub struct LoggerLayer;

impl<S> Layer<S> for LoggerLayer {
    type Service = Logger<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Logger { inner }
    }
}
