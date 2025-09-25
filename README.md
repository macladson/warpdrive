# warpdrive

A compatibility library for running Warp filters within Axum servers, enabling gradual migration from Warp to Axum. `warpdrive` is based on `warp` v0.3 and will not work for `warp` v0.4 or higher.

## Usage

Add to your `Cargo.toml`:
```toml
[dependencies]
warpdrive = "0.1.0"
axum = "0.8"
warp = "0.3"
```

## Example

```rust
use axum::{routing::get, Router};
use warpdrive::WarpService;
use warp::Filter;

#[tokio::main]
async fn main() {
    // Existing Warp routes
    let warp_routes = warp::path("api")
        .and(warp::get())
        .map(|| "Hello from Warp!")
        .boxed();

    // New Axum routes
    let app = Router::new()
        .route("/", get(|| async { "Hello from Axum!" }))
        .fallback_service(WarpService::new(warp_routes));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
