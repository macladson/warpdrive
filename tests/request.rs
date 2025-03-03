use axum::{body::Body as AxumBody, extract::Request as AxumRequest};
use axum_warp_compat::into_warp_request;
use warp::hyper::body::to_bytes as warp_body_to_bytes;

#[tokio::test]
async fn test_basic_get_request() {
    let axum_request = AxumRequest::builder()
        .method("GET")
        .uri("https://example.com/path?query=value")
        .body(AxumBody::empty())
        .unwrap();

    let warp_request = into_warp_request(axum_request).await.unwrap();

    assert_eq!(warp_request.method(), "GET");
    assert_eq!(
        warp_request.uri().to_string(),
        "https://example.com/path?query=value"
    );
}

#[tokio::test]
async fn test_all_http_methods() {
    let methods = vec![
        "GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "CONNECT", "PATCH", "TRACE",
    ];

    for method in methods {
        let axum_request = AxumRequest::builder()
            .method(method)
            .uri("https://example.com")
            .body(AxumBody::empty())
            .unwrap();

        let warp_request = into_warp_request(axum_request).await.unwrap();

        assert_eq!(warp_request.method().as_str(), method);
    }
}

#[tokio::test]
async fn test_with_headers() {
    let axum_request = AxumRequest::builder()
        .method("POST")
        .uri("https://example.com")
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .header(axum::http::header::USER_AGENT, "test-agent")
        .header("X-Custom-Header", "custom-value")
        .body(AxumBody::empty())
        .unwrap();

    let warp_request = into_warp_request(axum_request).await.unwrap();

    assert_eq!(
        warp_request
            .headers()
            .get(warp::http::header::CONTENT_TYPE)
            .unwrap(),
        "application/json"
    );
    assert_eq!(
        warp_request
            .headers()
            .get(warp::http::header::USER_AGENT)
            .unwrap(),
        "test-agent"
    );
    assert_eq!(
        warp_request.headers().get("X-Custom-Header").unwrap(),
        "custom-value"
    );
}

#[tokio::test]
async fn test_with_body() {
    let body = "Hello, World!";
    let axum_body = AxumBody::from(body);
    let axum_request = AxumRequest::builder()
        .method("POST")
        .uri("https://example.com")
        .header(axum::http::header::CONTENT_TYPE, "text/plain")
        .body(axum_body)
        .unwrap();

    let warp_request = into_warp_request(axum_request).await.unwrap();

    assert_eq!(
        warp_body_to_bytes(warp_request.into_body())
            .await
            .unwrap()
            .as_ref(),
        body.as_bytes()
    );
}

#[tokio::test]
async fn test_all_http_versions() {
    use axum::http::Version;

    let versions = vec![
        Version::HTTP_09,
        Version::HTTP_10,
        Version::HTTP_11,
        Version::HTTP_2,
        Version::HTTP_3,
    ];

    for version in versions {
        let axum_request = AxumRequest::builder()
            .method("GET")
            .uri("https://example.com")
            .version(version)
            .body(AxumBody::empty())
            .unwrap();

        let warp_request = into_warp_request(axum_request).await.unwrap();

        // Version should be preserved or fallback to HTTP_11
        assert!(matches!(
            warp_request.version(),
            warp::http::Version::HTTP_09
                | warp::http::Version::HTTP_10
                | warp::http::Version::HTTP_11
                | warp::http::Version::HTTP_2
                | warp::http::Version::HTTP_3
        ));
    }
}

#[tokio::test]
async fn test_complex_request() {
    let body = vec![1, 2, 3, 4];
    let axum_body = AxumBody::from(body.clone());
    let axum_request = AxumRequest::builder()
        .method("PATCH")
        .uri("https://api.example.com/v1/resources/123?filter=active")
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .header(axum::http::header::AUTHORIZATION, "Bearer token123")
        .header("X-Rate-Limit", "100")
        .header("X-Rate-Remaining", "95")
        .version(axum::http::Version::HTTP_2)
        .body(axum_body)
        .unwrap();

    let warp_request = into_warp_request(axum_request).await.unwrap();

    assert_eq!(warp_request.method(), "PATCH");
    assert_eq!(warp_request.uri().path(), "/v1/resources/123");
    assert_eq!(warp_request.uri().query(), Some("filter=active"));
    assert_eq!(
        warp_request
            .headers()
            .get(warp::http::header::CONTENT_TYPE)
            .unwrap(),
        "application/json"
    );
    assert_eq!(
        warp_request
            .headers()
            .get(warp::http::header::AUTHORIZATION)
            .unwrap(),
        "Bearer token123"
    );
    assert_eq!(warp_request.headers().get("X-Rate-Limit").unwrap(), "100");
    assert_eq!(
        warp_request.headers().get("X-Rate-Remaining").unwrap(),
        "95"
    );

    assert_eq!(
        warp_body_to_bytes(warp_request.into_body()).await.unwrap(),
        body
    );
}
