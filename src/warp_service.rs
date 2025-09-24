use std::{
    convert::Infallible,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{body::Body, extract::Request, http::StatusCode, response::Response};
use futures::Future;
use tower::Service;
use warp::{filters::BoxedFilter, Reply};

use crate::{into_axum_response, into_warp_request};

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
                Err(status) => create_error_response(status),
            };
            Ok(response)
        })
    }
}

async fn process_request_with_filter<T>(
    req: Request,
    filter: &BoxedFilter<(T,)>
) -> Result<Response, StatusCode>
where
    T: warp::Reply + Send + Sync + 'static,
{
    let warp_req = into_warp_request(req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut service = warp::service(filter.clone());

    let reply = service
        .call(warp_req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    into_axum_response(reply.into_response())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn create_error_response(status: StatusCode) -> Response {
    Response::builder()
        .status(status)
        .body(Body::from(format!("Error: {}", status)))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap()
        })
}
