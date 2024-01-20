use std::{convert::Infallible, future::Future, pin::Pin};

use futures::future::FutureExt;

use http::Response;
use hyper::{body::Body, Request};
use jsonrpsee::{
    server::{StopHandle, TowerService},
    RpcModule,
};
use serde_json::Value;
use tower::{layer::util::Identity, Service};
use ts_rs::TS;

use crate::handler::Handler;

pub struct Server {
    rpc_module: RpcModule<()>,
    handler_signatures: Vec<(String, (Vec<(String, String)>, String))>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            rpc_module: RpcModule::new(()),
            handler_signatures: Vec::new(),
        }
    }

    pub fn add<Params, Return>(
        mut self,
        route: impl AsRef<str>,
        handler: impl Handler<Params, Return>,
    ) -> Self
    where
        Params: TS,
        Return: TS,
    {
        self.handler_signatures.push((
            route.as_ref().to_string(),
            (handler.get_parameter_types(), handler.get_return_type()),
        ));

        dbg!(route.as_ref());

        self.rpc_module
            .register_async_method(route.as_ref().to_string().leak(), move |params, _ctx| {
                // TODO: Unsure if this is problematic. I believe it's only cloning the function
                // pointer, but would have to check somehow to know for sure.
                let handler = handler.clone();

                async move {
                    //
                    handler.call(params.parse::<Value>().unwrap())
                }
            })
            .unwrap();

        self
    }

    pub fn get_signatures(&self) -> String {
        let body = self
            .handler_signatures
            .iter()
            .map(|(route, (parameter_types, return_type))| {
                format!(
                    "{}: ({}) => Promise<{return_type}>",
                    route,
                    parameter_types
                        .into_iter()
                        .map(|(param, ty)| format!("{param}: {ty}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>();

        format!("{{ {} }}", body.join(", "))
    }

    pub fn create_service(self, stop_handle: StopHandle) -> ServerService {
        let svc_builder = jsonrpsee::server::Server::builder().to_service_builder();

        ServerService {
            service: svc_builder.build(self.rpc_module, stop_handle),
        }
    }
}

/// Wrapper service to convert `jsonrpsee`'s fallible errors into [`Infallible`] errors that are
/// required by Axum. Unsure if there's a better way to handle this in the future.
#[derive(Clone)]
pub struct ServerService {
    service: TowerService<Identity, Identity>,
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
