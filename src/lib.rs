//! A compatibility library for running Warp filters within Axum servers.
//!
//! This crate enables gradual migration from Warp to Axum by allowing existing
//! Warp routes to run alongside new Axum routes in the same server.
//!
//! # Example
//!
//! ```rust
//! use axum::{routing::get, Router};
//! use warpdrive::WarpService;
//! use warp::Filter;
//!
//! # #[tokio::main]
//! # async fn main() {
//! // Existing Warp routes
//! let warp_routes = warp::path("api")
//!     .and(warp::get())
//!     .map(|| "Hello from Warp!")
//!     .boxed();
//!
//! // Combine with Axum routes
//! let app: Router = Router::new()
//!     .route("/", get(|| async { "Hello from Axum!" }))
//!     .fallback_service(WarpService::new(warp_routes));
//! # }
//! ```
//!
//! ## Limitations
//!
//! - WebSockets are not supported, these should be migrated to Axum first.
//! - Some other advanced Warp features may not work.
//! - Some conversion overhead from converting `http::Request` and `http::Response` types.
//!
//! ## Error Handling
//!
//! WarpService acts as a transparent wrapper. The existing Warp rejection handling should work
//! exactly as before. It merely converts the pre-v1.0 `http::Response` into the Axum 0.8-compatible
//! v1.0 `http::Response` type.
//! The service only adds 500 errors in the extremely rare case of HTTP format conversion failures.

mod convert_request;
mod convert_response;
mod warp_service;

#[cfg(test)]
mod tests;

pub use warp_service::WarpService;
