use std::{collections::HashMap, convert::Infallible};

use axum::response::IntoResponse;
use futures::FutureExt;
use http::{HeaderValue, Method, Request, header};
use jsonrpsee::{
    RpcModule as JsonRpseeModule,
    server::{Server, ServerHandle, stop_channel, ws::is_upgrade_request},
};
use tower::{Service, ServiceBuilder, service_fn};

use crate::{
    reflection::handler::HandlerMeta,
    router::{RouterModule, RouterModuleHandler},
};

pub struct RpcModule<Ctx>(JsonRpseeModule<Ctx>);

impl<Ctx> RpcModule<Ctx> {
    pub fn new(ctx: Ctx) -> Self {
        Self(JsonRpseeModule::new(ctx))
    }

    pub fn into_module(self) -> JsonRpseeModule<Ctx> {
        self.0
    }

    pub fn into_service(
        self,
    ) -> (
        impl Service<
            Request<axum::body::Body>,
            Error = Infallible,
            Future = impl Send,
            Response = impl IntoResponse,
        > + Clone,
        ServerHandle,
    ) {
        let module = self.into_module();
        let (stop_handle, server_handle) = stop_channel();

        let mut tower_service = Server::builder()
            .set_http_middleware(ServiceBuilder::new().map_request(|mut req: Request<_>| {
                // Check if this is a GET request, and if it is convert it to a regular POST.
                if matches!(req.method(), &Method::GET) && !is_upgrade_request(&req) {
                    // Change this request into a regular POST request, and indicate that it should
                    // be a query.
                    *req.method_mut() = Method::POST;

                    // Update the headers.
                    let headers = req.headers_mut();
                    headers.insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );
                    headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));

                    // Convert the `input` field of the query string into the request body.
                    if let Some(body) = req
                        // Extract the query string.
                        .uri()
                        .query()
                        // Parse the query string.
                        .and_then(|query| serde_qs::from_str::<HashMap<String, String>>(query).ok())
                        // Take out the input.
                        .and_then(|mut query| query.remove("input"))
                        // URL decode the input.
                        .map(|input| urlencoding::decode(&input).unwrap_or_default().to_string())
                    {
                        // TODO: Replace `axum` with something else.
                        *req.body_mut() = axum::body::Body::from(body);
                    }
                };

                req
            }))
            .to_service_builder()
            .build(module, stop_handle);

        let service = service_fn(move |req| {
            let call = tower_service.call(req);

            async move {
                match call.await {
                    Ok(response) => Ok::<_, Infallible>(response),
                    // TODO: This should probably be an internal error
                    Err(_) => unreachable!(),
                }
            }
            .boxed()
        });

        (service, server_handle)
    }
}

impl<Ctx> RouterModule<Ctx> for RpcModule<Ctx> {
    type Handler = Handler<Ctx>;

    fn visit_handler(&mut self, path: &[&str], handler: &Self::Handler) {
        (handler.0)(&mut self.0, path.join("."));
    }
}

pub struct Handler<Ctx>(Box<dyn Fn(&mut JsonRpseeModule<Ctx>, String)>);
impl<Ctx> RouterModuleHandler<Ctx> for Handler<Ctx> {
    fn from_handler<
        F,
        MSig,
        MValue: crate::handler::marker::ResponseMarker,
        MReturn: crate::handler::marker::HandlerReturnMarker,
    >(
        handler: F,
        _meta: &'static HandlerMeta,
    ) -> Self
    where
        F: crate::RegisterableHandler<Ctx, MSig, MValue, MReturn>,
        F::Ctx: crate::FromRequestExtensions<Ctx>,
    {
        Self(Box::new(move |module, path| {
            handler.clone().register(module, path);
        }))
    }
}
