use std::str::FromStr;

use axum::body::Body as AxumBody;
use axum::extract::Request as AxumRequest;
use warp::http::{
    Request as WarpRequest, method::Method, uri::Uri, version::Version as WarpVersion,
};
use warp::hyper::body::Body as WarpBody;

pub async fn into_warp_request(
    axum_request: AxumRequest<AxumBody>,
) -> Result<WarpRequest<WarpBody>, String> {
    let (parts, body) = axum_request.into_parts();

    let method = Method::from_str(parts.method.as_ref())
        .map_err(|e| format!("Invalid method '{}': {}", parts.method, e))?;

    let uri = Uri::try_from(&parts.uri.to_string())
        .map_err(|e| format!("Invalid URI '{}': {}", parts.uri, e))?;

    let mut builder = WarpRequest::builder()
        .method(method)
        .uri(uri)
        .version(convert_version(parts.version));

    for (name, value) in parts.headers.iter() {
        builder = builder.header(name.as_str(), value.as_bytes())
    }

    builder
        .body(WarpBody::wrap_stream(body.into_data_stream()))
        .map_err(|e| format!("Failed to build Warp request: {}", e))
}

fn convert_version(version: axum::http::Version) -> WarpVersion {
    match version {
        axum::http::Version::HTTP_09 => WarpVersion::HTTP_09,
        axum::http::Version::HTTP_10 => WarpVersion::HTTP_10,
        axum::http::Version::HTTP_11 => WarpVersion::HTTP_11,
        axum::http::Version::HTTP_2 => WarpVersion::HTTP_2,
        axum::http::Version::HTTP_3 => WarpVersion::HTTP_3,
        // Default to 1.1 for compatibility.
        _ => WarpVersion::HTTP_11,
    }
}
