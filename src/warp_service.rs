use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    response::Response,
};
use crate::{into_axum_response, into_warp_request};
use futures::Future;
use std::{
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower::Service;
use warp::{Filter, Reply};

#[derive(Clone)]
pub struct WarpService {
    filter: Arc<warp::filters::BoxedFilter<(Box<dyn warp::Reply + Send + Sync>,)>>,
}

impl WarpService {
    pub fn new<T>(filter: warp::filters::BoxedFilter<(T,)>) -> Self
    where
        T: warp::Reply + Send + Sync + 'static,
    {
        let wrapped_filter = filter
            .map(|reply| Box::new(reply) as Box<dyn warp::Reply + Send + Sync>)
            .boxed();

        WarpService {
            filter: Arc::new(wrapped_filter),
        }
    }
}

impl Service<Request> for WarpService {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let filter = self.filter.clone();

        Box::pin(async move {
            let response = match process_request(req, (*filter).clone()).await {
                Ok(resp) => resp,
                Err(status) => Response::builder()
                    .status(status)
                    .body(Body::from(format!("Error: {}", status)))
                    .unwrap_or_else(|_| {
                        Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::empty())
                            .unwrap()
                    }),
            };

            Ok(response)
        })
    }
}

// Helper function to convert an Axum request to a Warp request,
// then handle the Warp response back into an Axum response.
async fn process_request<T>(
    req: Request,
    filter: warp::filters::BoxedFilter<(T,)>,
) -> Result<Response, StatusCode>
where
    T: Send + Sync + 'static,
    T: warp::Reply,
{
    let warp_req = into_warp_request(req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Process through Warp.
    let result = warp::service(filter)
        .call(warp_req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = result.into_response();

    into_axum_response(response)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
