//! Example showing how to configure CORS for both Axum and Warp routes in a single server.
//!
//! To run this example:
//! ```bash
//! cargo run --example cors
//! ```
//!
//! ```bash
//! # Axum route
//! # Preflight request
//! curl -i -X OPTIONS http://localhost:3000/axum/data \
//!   -H "Origin: http://example.com" \
//!   -H "Access-Control-Request-Method: POST"
//!
//! # Post request
//! curl -i -X POST http://localhost:3000/axum/data \
//!   -H "Origin: http://example.com" \
//!   -H "Content-Type: application/json" \
//!   -d '{"message":"hello"}'
//!
//! # Warp route
//! # Preflight request
//! curl -i -X OPTIONS http://localhost:3000/warp/data \
//!   -H "Origin: http://example.com" \
//!   -H "Access-Control-Request-Method: POST"
//!
//! # Post request
//! curl -i -X POST http://localhost:3000/warp/data \
//!   -H "Origin: http://example.com" \
//!   -H "Content-Type: application/json" \
//!   -d '{"message":"hello"}'
//! ```

use axum::{
    extract::Json,
    http::{HeaderValue, Method},
    routing::post,
    Router,
};
use axum_warp_compat::WarpService;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use warp::Filter;

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    message: String,
}

// Axum handler
async fn axum_handler(Json(payload): Json<Message>) -> Json<Message> {
    Json(Message {
        message: format!("Axum echo: {}", payload.message),
    })
}

// Warp handler
async fn warp_handler(payload: Message) -> Result<impl warp::Reply, Infallible> {
    Ok(warp::reply::json(&Message {
        message: format!("Warp echo: {}", payload.message),
    }))
}

#[tokio::main]
async fn main() {
    let allowed_origins = vec!["http://example.com"];
    let allowed_methods = vec![Method::GET, Method::POST, Method::OPTIONS];
    let allowed_headers = vec![
        axum::http::header::CONTENT_TYPE,
        axum::http::header::ACCEPT,
        axum::http::header::ORIGIN,
    ];

    let axum_cors = CorsLayer::new()
        .allow_origin(
            allowed_origins
                .iter()
                .map(|origin| HeaderValue::from_str(origin).unwrap())
                .collect::<Vec<_>>(),
        )
        .allow_methods(allowed_methods.clone())
        .allow_headers(allowed_headers.clone());

    let warp_cors = warp::cors()
        .allow_origins(allowed_origins.iter().copied())
        .allow_methods(allowed_methods.iter().map(|method| method.as_str()))
        .allow_headers(allowed_headers.iter().map(|header| header.as_str()));

    let warp_routes = warp::path("warp")
        .and(warp::path("data"))
        .and(warp::post())
        .and(warp::body::json())
        .and_then(warp_handler)
        .with(warp_cors)
        .boxed();

    let warp_service = WarpService::new(warp_routes);

    let app = Router::new()
        .route("/axum/data", post(axum_handler))
        .layer(axum_cors)
        .fallback_service(warp_service);

    // Start the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);
    println!("Available routes:");
    println!("  POST /axum/data");
    println!("  POST /warp/data");

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
