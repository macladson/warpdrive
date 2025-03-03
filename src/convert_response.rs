use axum::body::Body as AxumBody;
use axum::http::{version::Version, Response as AxumResponse};
use warp::http::Response as WarpResponse;
use warp::hyper::body::{to_bytes, HttpBody};

pub async fn into_axum_response<T>(
    warp_response: WarpResponse<T>,
) -> Result<AxumResponse<AxumBody>, String>
where
    T: HttpBody + Send + 'static,
{
    let (parts, body) = warp_response.into_parts();

    let mut builder = AxumResponse::builder()
        .status(axum::http::StatusCode::from_u16(parts.status.as_u16()).map_err(|e| e.to_string())?)
        .version(convert_version(parts.version));

    for (name, value) in parts.headers.iter() {
        builder = builder.header(name.as_str(), value.as_bytes());
    }

    let body_bytes = to_bytes(body)
        .await
        .map_err(|_| "Error converting warp body to bytes".to_string())?;
    let axum_body: AxumBody = body_bytes.into();

    builder.body(axum_body).map_err(|e| e.to_string())
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
