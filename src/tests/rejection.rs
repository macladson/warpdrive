// Tests to ensure that Warp rejection handling works unchanged through
// the service wrapper.
use axum::{body::Body as AxumBody, extract::Request as AxumRequest};
use tower::ServiceExt;
use warp::Filter;

use crate::warp_service::WarpService;

#[tokio::test]
async fn test_404_not_found() {
    let warp_filter = warp::path("exists")
        .and(warp::get())
        .map(|| "This route exists");

    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("GET")
        .uri("/does-not-exist")
        .body(AxumBody::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_405_method_not_allowed() {
    let warp_filter = warp::path("only-post")
        .and(warp::post())
        .map(|| "POST only");

    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("GET") // Wrong method
        .uri("/only-post")
        .body(AxumBody::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 405);
}

#[tokio::test]
async fn test_400_invalid_json() {
    #[derive(serde::Deserialize)]
    struct TestData {
        message: String,
    }

    let warp_filter = warp::path("json")
        .and(warp::post())
        .and(warp::body::json::<TestData>())
        .map(|data: TestData| format!("Got: {}", data.message));

    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("POST")
        .uri("/json")
        .header("content-type", "application/json")
        .body(AxumBody::from("invalid json content"))
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_400_missing_required_header() {
    let warp_filter = warp::path("auth")
        .and(warp::get())
        .and(warp::header::<String>("authorization"))
        .map(|_auth: String| "Authenticated");

    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("GET")
        .uri("/auth")
        // Missing required authorization header
        .body(AxumBody::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_400_invalid_query_parameter() {
    let warp_filter = warp::path("search")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, u32>>()) // Expects numeric values
        .map(|_params| "Search results");

    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("GET")
        .uri("/search?limit=not-a-number") // Invalid numeric parameter
        .body(AxumBody::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_400_invalid_path_parameter() {
    let warp_filter = warp::path("users")
        .and(warp::path::param::<u32>()) // Expects numeric user ID
        .and(warp::get())
        .map(|user_id: u32| format!("User: {}", user_id));

    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("GET")
        .uri("/users/not-a-number") // Invalid numeric path parameter
        .body(AxumBody::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 404); // Warp treats this as not found, not bad request
}

#[tokio::test]
async fn test_411_length_required() {
    #[derive(serde::Deserialize)]
    struct TestData {
        message: String,
    }

    let warp_filter = warp::path("upload")
        .and(warp::post())
        .and(warp::body::content_length_limit(10)) // Very small limit
        .and(warp::body::json::<TestData>())
        .map(|data: TestData| format!("Uploaded: {}", data.message));

    let service = WarpService::new(warp_filter.boxed());

    let large_json = serde_json::to_string(&serde_json::json!({
        "message": "This is a very long message that exceeds the content length limit"
    }))
    .unwrap();

    let request = AxumRequest::builder()
        .method("POST")
        .uri("/upload")
        .header("content-type", "application/json")
        .body(AxumBody::from(large_json))
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 411);
}

#[tokio::test]
async fn test_415_unsupported_media_type() {
    #[derive(serde::Deserialize)]
    struct TestData {
        message: String,
    }

    let warp_filter = warp::path("json-only")
        .and(warp::post())
        .and(warp::body::json::<TestData>())
        .map(|data: TestData| format!("Got JSON: {}", data.message));

    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("POST")
        .uri("/json-only")
        .header("content-type", "text/plain") // Wrong content type
        .body(AxumBody::from(r#"{"message": "test"}"#))
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 415);
}

#[tokio::test]
async fn test_multiple_possible_rejections() {
    // Test that the most specific rejection is returned
    let warp_filter = warp::path("strict")
        .and(warp::post())
        .and(warp::header::<String>("authorization"))
        .and(warp::body::json::<serde_json::Value>())
        .map(|_auth, _json| "Success");

    let service = WarpService::new(warp_filter.boxed());

    // Test wrong method (should be 405, not 400)
    let request = AxumRequest::builder()
        .method("GET") // Wrong method
        .uri("/strict")
        .header("authorization", "Bearer token")
        .header("content-type", "application/json")
        .body(AxumBody::from(r#"{"test": true}"#))
        .unwrap();

    let response = service.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), 405);

    // Test missing header (should be 400)
    let request = AxumRequest::builder()
        .method("POST")
        .uri("/strict")
        // Missing authorization header
        .header("content-type", "application/json")
        .body(AxumBody::from(r#"{"test": true}"#))
        .unwrap();

    let response = service.oneshot(request).await.unwrap();
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_rejection_preserves_warp_response_format() {
    let warp_filter = warp::path("test").and(warp::post()).map(|| "success");
    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("GET") // Wrong method
        .uri("/test")
        .body(AxumBody::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 405);

    // The response should maintain Warp's default error response format
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    // Warp typically returns "HTTP method not allowed" or similar
    assert!(!body.is_empty());
}
