use std::{
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{body::Body, extract::Request, http::StatusCode, response::Response};
use futures::Future;
use tower::Service;
use warp::{Filter, Reply};

use crate::{into_axum_response, into_warp_request};

#[derive(Clone)]
pub struct WarpService {
    filter: Arc<warp::filters::BoxedFilter<(Box<dyn warp::Reply + Send + Sync>,)>>,
}

impl WarpService {
    pub fn new<T>(filter: warp::filters::BoxedFilter<(T,)>) -> Self
    where
        T: warp::Reply + Send + Sync + 'static,
    {
        let boxed_filter = filter
            .map(|reply| Box::new(reply) as Box<dyn warp::Reply + Send + Sync>)
            .boxed();

        WarpService {
            filter: Arc::new(boxed_filter),
        }
    }

    async fn process_request(&self, req: Request) -> Result<Response, StatusCode> {
        let warp_req = into_warp_request(req)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let mut service = warp::service(self.filter.as_ref().clone());

        let response = service
            .call(warp_req)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .into_response();

        into_axum_response(response)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
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
        let warp_service = self.clone();

        Box::pin(async move {
            let response = match warp_service.process_request(req).await {
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
