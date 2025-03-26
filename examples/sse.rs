//! Example showing SSE (Server-Sent Events) streaming with both Axum and Warp in a single server.
//!
//! To run this example:
//! ```bash
//! cargo run --example sse
//! ```
//!
//! ```bash
//! # Axum SSE
//! curl http://localhost:3000/axum/sse
//!
//! # Warp SSE
//! curl http://localhost:3000/warp/sse
//! ```

use axum::{
    response::sse::{Event as AxumEvent, KeepAlive, Sse},
    routing::get,
    Router,
};
use axum_warp_compat::WarpService;
use chrono::Local;
use futures::Stream;
use std::{convert::Infallible, net::SocketAddr, time::Duration};
use tokio::net::TcpListener;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use warp::{filters::sse::Event as WarpEvent, Filter};

fn timestamp_stream() -> impl Stream<Item = String> {
    IntervalStream::new(tokio::time::interval(Duration::from_secs(1)))
        .map(|_| Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
}

async fn axum_stream() -> Sse<impl Stream<Item = Result<AxumEvent, Infallible>>> {
    let stream = timestamp_stream()
        .map(|timestamp| Ok(AxumEvent::default().data(format!("Axum time: {}", timestamp))));

    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn warp_stream() -> impl Stream<Item = Result<WarpEvent, Infallible>> {
    let stream = timestamp_stream()
        .map(|timestamp| Ok(WarpEvent::default().data(format!("Warp time: {}", timestamp))));

    stream
}

#[tokio::main]
async fn main() {
    let warp_sse = warp::path("warp")
        .and(warp::path("sse"))
        .and(warp::get())
        .map(|| warp::sse::reply(warp::sse::keep_alive().stream(warp_stream())))
        .boxed();

    let warp_service = WarpService::new(warp_sse);

    let app = Router::new()
        .route("/axum/sse", get(axum_stream))
        .fallback_service(warp_service);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);
    println!("Available SSE endpoints:");
    println!("  GET /axum/sse");
    println!("  GET /warp/sse");

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
