use std::{convert::Infallible, future::Future, pin::Pin, rc::Rc};

use futures::future::FutureExt;

use http::Response;
use hyper::{body::Body, Request};
use jsonrpsee::{
    server::{StopHandle, TowerService},
    RpcModule,
};
use matchit::Router;
use serde_json::Value;
use tower::{layer::util::Identity, Service};
use ts_rs::TS;

use crate::handler::{ErasedHandler, Handler, MakeErasedHandler};

pub struct Server {
    /// Keep all handlers in a Vec in-order to infer signatuers for server
    signature_routes: Vec<(String, Rc<Box<dyn ErasedHandler>>)>,

    /// Only allow router to store references to handlers
    router: Router<Rc<Box<dyn ErasedHandler>>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            signature_routes: Vec::new(),
            router: Router::new(),
        }
    }

    pub fn add<Params, Return>(
        mut self,
        route: impl AsRef<str>,
        handler: impl Handler<Params, Return> + 'static,
    ) -> Self
    where
        Params: TS,
        Return: TS,
    {
        let handler = Rc::new(Box::new(MakeErasedHandler {
            handler,
            do_call: |handler, params| handler.call(params),
            get_signature: |handler| (handler.get_parameter_types(), handler.get_return_type()),
        }) as Box<dyn ErasedHandler>);

        self.signature_routes
            .push((route.as_ref().to_string(), handler.clone()));

        self.router
            .insert(route.as_ref().to_string(), handler)
            .unwrap();

        self
    }

    pub fn call(&self, route: impl AsRef<str>, parameters: Value) -> Value {
        self.router
            .at(route.as_ref())
            .unwrap()
            .value
            .clone_box()
            .call(parameters)
    }

    pub fn get_signatures(&self) -> String {
        let body = self
            .signature_routes
            .iter()
            .map(|(route, handler)| {
                let (parameter_types, return_type) = handler.get_signature();

                let signature = format!(
                    "{}: ({}) => {return_type}",
                    route,
                    parameter_types
                        .into_iter()
                        .map(|(param, ty)| format!("{param}: {ty}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );

                signature
            })
            .collect::<Vec<_>>();

        format!("{{ {} }}", body.join(", "))
    }

    pub fn create_service(&self, stop_handle: StopHandle) -> ServerService {
        let svc_builder = jsonrpsee::server::Server::builder().to_service_builder();

        let mut module = RpcModule::new(());
        module
            .register_method("test", |_params, _ctx| {
                println!("called");

                "working"
            })
            .unwrap();

        ServerService {
            service: svc_builder.clone().build(module.clone(), stop_handle),
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
