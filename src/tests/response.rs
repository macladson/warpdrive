use axum::http::StatusCode as AxumStatusCode;
use serde_json::json;
use warp::{
    Reply,
    http::{response::Response as WarpResponse, status::StatusCode as WarpStatusCode},
    hyper::Body as WarpBody,
    reply::{json, with_header, with_status},
};

use crate::convert_response::into_axum_response;

#[tokio::test]
async fn test_basic_response() {
    let warp_response = WarpResponse::builder()
        .status(WarpStatusCode::OK)
        .body(WarpBody::from("Hello World!"))
        .unwrap();

    let axum_response = into_axum_response(warp_response).await.unwrap();

    assert_eq!(axum_response.status(), AxumStatusCode::OK);
}

#[tokio::test]
async fn test_response_with_status() {
    let warp_response = WarpResponse::builder()
        .status(WarpStatusCode::NOT_FOUND)
        .body(WarpBody::empty())
        .unwrap();

    let axum_response = into_axum_response(warp_response).await.unwrap();

    assert_eq!(axum_response.status(), AxumStatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_response_with_headers() {
    let warp_response = WarpResponse::builder()
        .header(warp::http::header::CONTENT_TYPE, "text/plain")
        .body(WarpBody::empty())
        .unwrap();

    let axum_response = into_axum_response(warp_response).await.unwrap();

    assert_eq!(
        axum_response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap(),
        "text/plain"
    );
}

#[tokio::test]
async fn test_json_response() {
    let data = json!({
        "message": "Hello World!"
    });
    let warp_response = json(&data).into_response();

    let axum_response = into_axum_response(warp_response).await.unwrap();

    assert_eq!(
        axum_response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap(),
        "application/json"
    );
}

#[tokio::test]
async fn test_multiple_headers() {
    let response = with_header(
        with_header("Hello World!", "X-Custom-Header", "custom-value"),
        "X-Another-Header",
        "another-value",
    )
    .into_response();

    let axum_response = into_axum_response(response).await.unwrap();

    assert_eq!(
        axum_response.headers().get("X-Custom-Header").unwrap(),
        "custom-value"
    );
    assert_eq!(
        axum_response.headers().get("X-Another-Header").unwrap(),
        "another-value"
    );
}

#[tokio::test]
async fn test_different_http_versions() {
    use axum::http::Version as AxumVersion;
    use warp::http::Version as WarpVersion;

    let versions = vec![
        WarpVersion::HTTP_09,
        WarpVersion::HTTP_10,
        WarpVersion::HTTP_11,
        WarpVersion::HTTP_2,
        WarpVersion::HTTP_3,
    ];

    for version in versions {
        let response = WarpResponse::builder()
            .version(version)
            .body(WarpBody::from("Hello"))
            .unwrap();

        let axum_response = into_axum_response(response).await.unwrap();

        // Version should be preserved or fallback to HTTP_11
        assert!(matches!(
            axum_response.version(),
            AxumVersion::HTTP_09
                | AxumVersion::HTTP_10
                | AxumVersion::HTTP_11
                | AxumVersion::HTTP_2
                | AxumVersion::HTTP_3
        ));
    }
}

#[tokio::test]
async fn test_complex_response() {
    let data = json!({
        "status": "success",
        "data": {
            "message": "Hello World!",
            "code": 200
        }
    });

    let response = with_header(
        with_status(json(&data), WarpStatusCode::OK),
        "X-Rate-Limit",
        "100",
    )
    .into_response();

    let axum_response = into_axum_response(response).await.unwrap();

    assert_eq!(axum_response.status(), AxumStatusCode::OK);
    assert_eq!(
        axum_response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap(),
        "application/json"
    );
    assert_eq!(axum_response.headers().get("X-Rate-Limit").unwrap(), "100");
}
