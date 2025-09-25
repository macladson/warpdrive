//! Example showing how to combine Axum and Warp routes in a single server.
//!
//! To run this example:
//! ```bash
//! cargo run --example mixed_server
//! ```
//!
//! ```bash
//! # Axum routes
//! curl http://localhost:3000/axum
//! curl -X POST -H "Content-Type: application/json" -d '{"content":"test"}' http://localhost:3000/axum/echo
//!
//! # Warp routes
//! curl http://localhost:3000/warp
//! curl -X POST -H "Content-Type: application/json" -d '{"content":"test"}' http://localhost:3000/warp/echo
//! ```

use std::{convert::Infallible, net::SocketAddr};

use axum::{
    Router,
    extract::Json,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use warp::Filter;
use warpdrive::WarpService;

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    content: String,
}

// Axum route handlers.
async fn axum_hello() -> &'static str {
    "Hello from Axum!"
}

async fn axum_echo(Json(message): Json<Message>) -> Json<Message> {
    Json(Message {
        content: format!("Axum received: {}", message.content),
    })
}

// Warp route handlers.
async fn warp_hello() -> Result<impl warp::Reply, Infallible> {
    Ok(warp::reply::html("Hello from Warp!"))
}

async fn warp_echo(message: Message) -> Result<impl warp::Reply, Infallible> {
    Ok(warp::reply::json(&Message {
        content: format!("Warp received: {}", message.content),
    }))
}

#[tokio::main]
async fn main() {
    let warp_hello_route = warp::path("warp")
        .and(warp::get())
        .and(warp::path::end())
        .and_then(warp_hello);

    let warp_echo_route = warp::path("warp")
        .and(warp::path("echo"))
        .and(warp::post())
        .and(warp::body::json())
        .and_then(warp_echo);

    let warp_routes = warp_hello_route.or(warp_echo_route).boxed();

    let warp_service = WarpService::new(warp_routes);

    // Define Axum router with both Axum and Warp routes.
    let app = Router::new()
        .route("/axum", get(axum_hello))
        .route("/axum/echo", post(axum_echo))
        // Use Warp service as fallback.
        .fallback_service(warp_service);

    // Start the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);
    println!("Available routes:");
    println!("  GET  /axum");
    println!("  POST /axum/echo");
    println!("  GET  /warp");
    println!("  POST /warp/echo");

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
