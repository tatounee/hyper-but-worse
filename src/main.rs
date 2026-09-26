#![feature(impl_trait_in_assoc_type)]
#![feature(trim_prefix_suffix)]
#![feature(normalize_lexically)]
#![warn(clippy::pedantic)]
// #![allow(warnings)]

use color_eyre::{Result, eyre::Context};
use dotenvy::dotenv;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::services::{
    CounterService, CounterStream, DatabaseLayer, HelloService, Router, StaticFile,
    sse::ServerSendEvent,
};

mod app;
mod body;
mod config;
mod error;
mod services;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv()?;

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(EnvFilter::from_default_env())
        .init();

    color_eyre::install()?;

    let (host, port) = config::from_env()?;
    info!("Starting application on http://[{host}]:{port}");

    let listener = TcpListener::bind((host, port)).await?;

    let static_dir =
        std::env::var("STATIC_DIR").wrap_err("reading STATIC_DIR environement variable")?;

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
        );

    app::run(listener, router).await?;

    Ok(())
}
