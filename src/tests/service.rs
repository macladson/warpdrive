use axum::{body::Body as AxumBody, extract::Request as AxumRequest};
use tower::ServiceExt;
use warp::Filter;

use crate::warp_service::WarpService;

#[tokio::test]
async fn test_basic_get_request() {
    let warp_filter = warp::path("hello")
        .and(warp::get())
        .map(|| "Hello from Warp!");

    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("GET")
        .uri("/hello")
        .body(AxumBody::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 200);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, "Hello from Warp!");
}

#[tokio::test]
async fn test_post_request_with_json() {
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct TestData {
        message: String,
        count: u32,
    }

    let warp_filter = warp::path("api")
        .and(warp::path("data"))
        .and(warp::post())
        .and(warp::body::json())
        .map(|data: TestData| {
            warp::reply::json(&TestData {
                message: format!("Received: {}", data.message),
                count: data.count + 1,
            })
        });

    let service = WarpService::new(warp_filter.boxed());

    let request_data = TestData {
        message: "test".to_string(),
        count: 5,
    };

    let request = AxumRequest::builder()
        .method("POST")
        .uri("/api/data")
        .header("content-type", "application/json")
        .body(AxumBody::from(
            serde_json::to_string(&request_data).unwrap(),
        ))
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_data: TestData = serde_json::from_slice(&body).unwrap();

    assert_eq!(response_data.message, "Received: test");
    assert_eq!(response_data.count, 6);
}

#[tokio::test]
async fn test_query_parameters() {
    let warp_filter = warp::path("search")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .map(|params: std::collections::HashMap<String, String>| {
            let query = params.get("q").map_or("empty", |s| s);
            let limit = params.get("limit").map_or("10", |s| s);
            format!("Query: {}, Limit: {}", query, limit)
        });

    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("GET")
        .uri("/search?q=rust&limit=5")
        .body(AxumBody::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 200);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, "Query: rust, Limit: 5");
}

#[tokio::test]
async fn test_path_parameters() {
    let warp_filter = warp::path("users")
        .and(warp::path::param::<u32>())
        .and(warp::path("posts"))
        .and(warp::path::param::<u32>())
        .and(warp::get())
        .map(|user_id: u32, post_id: u32| format!("User {} Post {}", user_id, post_id));

    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("GET")
        .uri("/users/123/posts/456")
        .body(AxumBody::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 200);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, "User 123 Post 456");
}

#[tokio::test]
async fn test_headers() {
    let warp_filter = warp::path("protected")
        .and(warp::get())
        .and(warp::header::<String>("authorization"))
        .and(warp::header::<String>("user-agent"))
        .map(|auth: String, user_agent: String| format!("Auth: {}, UA: {}", auth, user_agent));

    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("GET")
        .uri("/protected")
        .header("authorization", "Bearer token123")
        .header("user-agent", "test-client/1.0")
        .body(AxumBody::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 200);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, "Auth: Bearer token123, UA: test-client/1.0");
}

#[tokio::test]
async fn test_multiple_http_methods() {
    let get_filter = warp::path("resource")
        .and(warp::get())
        .map(|| "GET response");

    let post_filter = warp::path("resource")
        .and(warp::post())
        .map(|| "POST response");

    let put_filter = warp::path("resource")
        .and(warp::put())
        .map(|| "PUT response");

    let combined_filter = get_filter.or(post_filter).or(put_filter);

    let service = WarpService::new(combined_filter.boxed());

    // Test GET
    let get_request = AxumRequest::builder()
        .method("GET")
        .uri("/resource")
        .body(AxumBody::empty())
        .unwrap();

    let response = service.clone().oneshot(get_request).await.unwrap();
    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, "GET response");

    // Test POST
    let post_request = AxumRequest::builder()
        .method("POST")
        .uri("/resource")
        .body(AxumBody::empty())
        .unwrap();

    let response = service.clone().oneshot(post_request).await.unwrap();
    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, "POST response");

    // Test PUT
    let put_request = AxumRequest::builder()
        .method("PUT")
        .uri("/resource")
        .body(AxumBody::empty())
        .unwrap();

    let response = service.oneshot(put_request).await.unwrap();
    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, "PUT response");
}

#[tokio::test]
async fn test_custom_status_and_headers() {
    let warp_filter = warp::path("custom").and(warp::get()).map(|| {
        warp::reply::with_status(
            warp::reply::with_header("Custom response", "x-custom-header", "custom-value"),
            warp::http::StatusCode::CREATED,
        )
    });

    let service = WarpService::new(warp_filter.boxed());

    let request = AxumRequest::builder()
        .method("GET")
        .uri("/custom")
        .body(AxumBody::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 201);
    assert_eq!(
        response.headers().get("x-custom-header").unwrap(),
        "custom-value"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, "Custom response");
}
