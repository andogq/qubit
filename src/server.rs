use std::{convert::Infallible, future::Future, pin::Pin};

use futures::future::FutureExt;

use http::Response;
use hyper::{body::Body, Request};
use jsonrpsee::server::TowerService;
use tower::{layer::util::Identity, Service};

/// Wrapper service to convert `jsonrpsee`'s fallible errors into [`Infallible`] errors that are
/// required by Axum. Unsure if there's a better way to handle this in the future.
#[derive(Clone)]
pub struct ServerService {
    pub(crate) service: TowerService<Identity, Identity>,
}

impl Service<Request<Body>> for ServerService {
    type Response = Response<Body>;
    type Error = Infallible;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        // Convert any fallible errors into infallible errors
        self.service.poll_ready(cx).map(|result| match result {
            Ok(result) => Ok(result),
            Err(e) => {
                eprintln!("jsonrpsee error occurred when it should be infallible");
                eprintln!("{e:?}");

                Ok(())
            }
        })
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        Box::pin(self.service.call(req).map(|result| match result {
            Ok(result) => Ok(result),
            Err(e) => {
                eprintln!("jsonrpsee error occurred when it should be infallible");
                eprintln!("{e:?}");

                Ok(Response::builder()
                    .status(500)
                    .body(Body::from("uncaught internal error"))
                    .unwrap_or(Response::default()))
            }
        }))
    }
}
