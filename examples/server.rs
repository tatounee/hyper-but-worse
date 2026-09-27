#![feature(impl_trait_in_assoc_type)]

use color_eyre::Result;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use hyper_but_worse::{
    app,
    services::{Redirect, Router, StaticFile, database::DatabaseLayer, sse::ServerSendEvent},
};

mod example_services;

use example_services::{CounterService, CounterStream, HelloService};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::from_path("examples/.env")?;

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(EnvFilter::from_default_env())
        .init();

    color_eyre::install()?;

    let host = "::1";
    let port = 8800;
    info!("Starting application on http://[{host}]:{port}");

    let listener = TcpListener::bind((host, port)).await?;

    let static_dir = "examples/static";

    let hello = ServiceBuilder::new()
        .layer(DatabaseLayer)
        .service(HelloService);

    let router = Router::new()
        .layer(DatabaseLayer)
        .route("/hello", hello)
        .route("/counter", CounterService)
        .route("/static", StaticFile::new(static_dir)?)
        .route(
            "/sse/counter",
            ServerSendEvent::new(CounterStream::default()),
        )
        .route("/", Redirect::new("/static"));

    app::run(listener, router).await?;

    Ok(())
}
