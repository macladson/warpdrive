use crate::{into_axum_response, into_warp_request};
use axum::{body::Body, extract::Request, http::StatusCode, response::Response};
use futures::Future;
use std::{
    convert::Infallible,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tower::Service;
use warp::{Filter, Reply};

type WrappedWarpService = dyn warp::hyper::service::Service<
        warp::hyper::Request<warp::hyper::Body>,
        Response = warp::hyper::Response<warp::hyper::Body>,
        Error = Infallible,
        Future = Pin<
            Box<
                dyn Future<Output = Result<warp::hyper::Response<warp::hyper::Body>, Infallible>>
                    + Send,
            >,
        >,
    > + Send
    + Sync;

struct BoxedWarpService<S>(S);

impl<S> Service<warp::hyper::Request<warp::hyper::Body>> for BoxedWarpService<S>
where
    S: Service<
            warp::hyper::Request<warp::hyper::Body>,
            Response = warp::hyper::Response<warp::hyper::Body>,
        > + Send
        + Sync,
    S::Future: Send + 'static,
{
    type Response = warp::hyper::Response<warp::hyper::Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // WarpService always returns Ok for poll_ready.
        self.0.poll_ready(cx).map(|r| {
            match r {
                Ok(()) => Ok(()),
                // Unreachable since WarpService never returns an error.
                Err(_) => unreachable!("Internal service never returns poll_ready error"),
            }
        })
    }

    fn call(&mut self, req: warp::hyper::Request<warp::hyper::Body>) -> Self::Future {
        let future = self.0.call(req);
        Box::pin(async move {
            match future.await {
                Ok(response) => Ok(response),
                // Unreachable since WarpService never returns an error.
                Err(_) => unreachable!("Internal service never returns call error"),
            }
        })
    }
}

#[derive(Clone)]
pub struct WarpService {
    service: Arc<Mutex<WrappedWarpService>>,
}

impl WarpService {
    pub fn new<T>(filter: warp::filters::BoxedFilter<(T,)>) -> Self
    where
        T: warp::Reply + Send + Sync + 'static,
    {
        let wrapped_filter = filter
            .map(|reply| Box::new(reply) as Box<dyn warp::Reply + Send + Sync>)
            .boxed();

        let service = warp::service(wrapped_filter);
        let boxed_service = BoxedWarpService(service);

        WarpService {
            service: Arc::new(Mutex::new(boxed_service)),
        }
    }

    async fn process_request(&self, req: Request) -> Result<Response, StatusCode> {
        let warp_req = into_warp_request(req)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let future = self
            .service
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .call(warp_req);

        let response = future
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
