use axum::body::Body as AxumBody;
use axum::http::{Response as AxumResponse, version::Version};
use futures::TryStreamExt;
use warp::http::Response as WarpResponse;
use warp::hyper::body::Body as WarpBody;

pub async fn into_axum_response(
    warp_response: WarpResponse<WarpBody>,
) -> Result<AxumResponse<AxumBody>, String> {
    let (parts, body) = warp_response.into_parts();

    let status_code = axum::http::StatusCode::from_u16(parts.status.as_u16())
        .map_err(|e| format!("Invalid status code {}: {}", parts.status.as_u16(), e))?;

    let mut builder = AxumResponse::builder()
        .status(status_code)
        .version(convert_version(parts.version));

    for (name, value) in parts.headers.iter() {
        builder = builder.header(name.as_str(), value.as_bytes());
    }

    builder
        .body(AxumBody::from_stream(body.into_stream()))
        .map_err(|e| format!("Failed to build Axum response: {}", e))
}

fn convert_version(version: warp::http::Version) -> Version {
    match version {
        warp::http::Version::HTTP_09 => Version::HTTP_09,
        warp::http::Version::HTTP_10 => Version::HTTP_10,
        warp::http::Version::HTTP_11 => Version::HTTP_11,
        warp::http::Version::HTTP_2 => Version::HTTP_2,
        warp::http::Version::HTTP_3 => Version::HTTP_3,
        // Default to 1.1 for compatibility.
        _ => Version::HTTP_11,
    }
}
