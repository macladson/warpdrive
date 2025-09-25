//! Example showing layered fallback routing with path prefixes.
//! This allows you run a warp service in addition to a separate bespoke fallback service.
//!
//! Routing order:
//! 1. Axum routes: `/axum/*`
//! 2. Warp routes: `/warp/*`
//! 3. Final fallback: everything else
//!
//! To run this example:
//! ```bash
//! cargo run --example nested_fallback
//! ```
//!
//! Test commands:
//! ```bash
//! # Axum route
//! curl http://localhost:3000/axum/hello
//!
//! # Warp routes
//! curl http://localhost:3000/warp/hello
//!
//! # Final fallback
//! curl http://localhost:3000/anything/else
//! ```

use std::{convert::Infallible, net::SocketAddr};

use axum::{Router, response::Response, routing::get};
use tokio::net::TcpListener;
use tower::Service;
use warp::Filter;
use warpdrive::WarpService;

async fn axum_hello() -> &'static str {
    "Hello from Axum!"
}

async fn warp_hello() -> Result<impl warp::Reply, Infallible> {
    Ok("Hello from Warp!")
}

#[derive(Clone)]
struct FinalFallback;

impl Service<axum::extract::Request> for FinalFallback {
    type Response = Response;
    type Error = Infallible;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: axum::extract::Request) -> Self::Future {
        let path = req.uri().path().to_string();
        let response = Response::builder()
            .body(format!("Final fallback caught: {}", path).into())
            .unwrap();
        std::future::ready(Ok(response))
    }
}

#[tokio::main]
async fn main() {
    let warp_routes = warp::path("hello")
        .and(warp::get())
        .and_then(warp_hello)
        .boxed();

    let warp_service = WarpService::new(warp_routes);

    // Create layered router
    let app = Router::new()
        .route("/axum/hello", get(axum_hello)) // Layer 1: Axum
        .nest_service("/warp", warp_service) // Layer 2: Warp
        .fallback_service(FinalFallback); // Layer 3: Fallback

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);
    println!("Routes:");
    println!("  /axum/hello  -> Axum");
    println!("  /warp/hello  -> Warp");
    println!("  /*           -> Fallback");

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
