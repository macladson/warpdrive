//! Example showing how to combine Axum and Warp routes in a single server.
//!
//! To run this example:
//! ```bash
//! cargo run --example mixed_server
//! ```
//!
//! Test with curl:
//! ```bash
//! # Test Axum routes
//! curl http://localhost:3000/axum
//! curl -X POST -H "Content-Type: application/json" -d '{"content":"test"}' http://localhost:3000/axum/echo
//!
//! # Test Warp routes
//! curl http://localhost:3000/warp
//! curl -X POST -H "Content-Type: application/json" -d '{"content":"test"}' http://localhost:3000/warp/echo
//! ```

use axum::{
    body::Body,
    extract::{Json, Request},
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use axum_warp_compatibility::{into_axum_response, into_warp_request};
use futures::Future;
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::net::TcpListener;
use tower::Service;
use warp::{Filter, Reply};

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

#[derive(Clone)]
struct WarpService {
    filter: Arc<warp::filters::BoxedFilter<(Box<dyn warp::Reply + Send + Sync>,)>>,
}

impl WarpService {
    fn new<T>(filter: warp::filters::BoxedFilter<(T,)>) -> Self
    where
        T: warp::Reply + Send + Sync + 'static,
    {
        let wrapped_filter = filter
            .map(|reply| Box::new(reply) as Box<dyn warp::Reply + Send + Sync>)
            .boxed();

        WarpService {
            filter: Arc::new(wrapped_filter),
        }
    }
}

impl Service<Request> for WarpService {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let filter = self.filter.clone();

        Box::pin(async move {
            let response = match process_request(req, (*filter).clone()).await {
                Ok(resp) => resp,
                Err(status) => Response::builder()
                    .status(status)
                    .body(Body::from(format!("Error: {}", status)))
                    .unwrap_or_else(|_| {
                        Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::empty())
                            .unwrap()
                    }),
            };

            Ok(response)
        })
    }
}

// Helper function to convert an Axum request to a Warp request,
// then handle the Warp response back into an Axum response.
async fn process_request<T>(
    req: Request,
    filter: warp::filters::BoxedFilter<(T,)>,
) -> Result<Response, StatusCode>
where
    T: Send + Sync + 'static,
    T: warp::Reply,
{
    let warp_req = into_warp_request(req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Process through Warp.
    let result = warp::service(filter)
        .call(warp_req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = result.into_response();

    into_axum_response(response)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
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
    println!("  GET  /axum          - Axum hello endpoint");
    println!("  POST /axum/echo     - Axum echo endpoint");
    println!("  GET  /warp          - Warp hello endpoint");
    println!("  POST /warp/echo     - Warp echo endpoint");

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
