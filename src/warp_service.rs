use std::{
    convert::Infallible,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{body::Body, extract::Request, response::Response};
use futures::Future;
use tower::Service;
use warp::{Reply, filters::BoxedFilter};

use crate::{convert_request::into_warp_request, convert_response::into_axum_response};

/// A Tower service that wraps Warp filters to run within Axum servers.
///
/// `WarpService` converts between Axum and Warp request/response types,
/// allowing Warp filters to be used as Axum services. This is particularly
/// useful for gradual migration from Warp to Axum.
///
/// # Example
///
/// ```rust
/// use warpdrive::WarpService;
/// use warp::Filter;
///
/// let warp_filter = warp::path("users")
///     .and(warp::path::param::<u32>())
///     .and(warp::get())
///     .map(|id: u32| format!("User {}", id));
///
/// let service = WarpService::new(warp_filter.boxed());
/// ```
pub struct WarpService<T = Box<dyn warp::Reply + Send + Sync>> {
    filter: Arc<BoxedFilter<(T,)>>,
    _phantom: PhantomData<T>,
}

impl<T> Clone for WarpService<T> {
    fn clone(&self) -> Self {
        WarpService {
            filter: Arc::clone(&self.filter),
            _phantom: PhantomData,
        }
    }
}

impl<T> WarpService<T>
where
    T: warp::Reply + Send + Sync + 'static,
{
    /// Creates a new `WarpService` from a Warp filter.
    ///
    /// The filter should be boxed using `.boxed()` before being passed to this method.
    ///
    /// # Example
    ///
    /// ```rust
    /// use warpdrive::WarpService;
    /// use warp::Filter;
    ///
    /// let json_filter = warp::path("api")
    ///     .and(warp::get())
    ///     .map(|| warp::reply::json(&"Hello"));
    ///
    /// let service = WarpService::new(json_filter.boxed());
    /// ```
    pub fn new(filter: BoxedFilter<(T,)>) -> Self {
        WarpService {
            filter: Arc::new(filter),
            _phantom: PhantomData,
        }
    }
}

impl<T> Service<Request> for WarpService<T>
where
    T: warp::Reply + Send + Sync + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let filter = Arc::clone(&self.filter);

        Box::pin(async move {
            let response = match process_request_with_filter(req, &filter).await {
                Ok(resp) => resp,
                Err(err) => create_conversion_error_response(err),
            };
            Ok(response)
        })
    }
}

async fn process_request_with_filter<T>(
    req: Request,
    filter: &BoxedFilter<(T,)>,
) -> Result<Response, String>
where
    T: warp::Reply + Send + Sync + 'static,
{
    let warp_req = into_warp_request(req).await?;

    let mut service = warp::service(filter.clone());

    let warp_response = match service.call(warp_req).await {
        Ok(reply) => reply.into_response(),
        Err(rejection) => rejection.into_response(),
    };

    into_axum_response(warp_response).await
}

// This only runs in the unlikely event of a conversion error.
fn create_conversion_error_response(err: String) -> Response {
    let status = axum::http::StatusCode::INTERNAL_SERVER_ERROR;

    Response::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(Body::from(format!("Conversion error: {}", err)))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(status)
                .body(Body::from("Critical error"))
                .unwrap()
        })
}
