use std::{ collections::BTreeMap, convert::Infallible, fs, path::Path};

use futures::FutureExt;
use http::Request;
use hyper::{service::service_fn, Body};
use jsonrpsee::{server::StopHandle, RpcModule};
use tower::Service;

use crate::{
    handler::{Handler, HandlerCallbacks},
    rpc_builder::RpcBuilder,
};

/// Router for the RPC server. Can have different handlers attached to it, as well as nested
/// routers in order to create a hierarchy. It is also capable of generating its own type, suitable
/// for consumption by a TypeScript client.
#[derive(Clone)]
pub struct Router<Ctx> {
    nested_routers: Vec<(&'static str, Router<Ctx>)>,
    handlers: Vec<HandlerCallbacks<Ctx>>,
}

impl<Ctx> Router<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            nested_routers: Vec::new(),
            handlers: Vec::new(),
        }
    }

    pub fn handler<H: Handler<Ctx>>(mut self, handler: H) -> Self {
        self.handlers.push(HandlerCallbacks::from_handler(handler));

        self
    }

    pub fn nest(mut self, namespace: &'static str, router: Router<Ctx>) -> Self {
        self.nested_routers.push((namespace, router));

        self
    }

    pub fn add_dependencies(&self, dependencies: &mut BTreeMap<String, String>) {
        // Add all handler dependencies
        self.handlers
            .iter()
            .for_each(|handler| (handler.add_dependencies)(dependencies));

        // Add dependencies for nested routers
        self.nested_routers
            .iter()
            .for_each(|(_, router)| router.add_dependencies(dependencies));
    }

    pub fn get_type(&self) -> String {
        let mut handlers = self
            .handlers
            .iter()
            .map(|handler| (handler.get_type)())
            .map(|handler_type| format!("{}: {}", handler_type.name, handler_type.signature))
            .collect::<Vec<_>>();

        self.nested_routers
            .iter()
            .map(|(namespace, router)| (namespace, router.get_type()))
            .for_each(|(namespace, router_type)| {
                handlers.push(format!("{namespace}: {router_type}"));
            });

        // Generate the router type
        let router_type = format!("{{ {} }}", handlers.join(", "));

        router_type
    }

    pub fn write_type_to_file(&self, path: impl AsRef<Path>) {
        let mut dependencies = BTreeMap::new();
        self.add_dependencies(&mut dependencies);
        let dependencies = dependencies
            .into_iter()
            .map(|(name, ty)| format!("type {name} = {ty};"))
            .collect::<Vec<_>>()
            .join("\n");

        let router = self.get_type();
        let router = format!("export type Server = {router};");

        fs::write(path, format!("{dependencies}\n{router}")).unwrap();
    }

    pub fn build_rpc_module(self, ctx: Ctx, namespace: Option<&'static str>) -> RpcModule<Ctx> {
        let mut rpc_module = self
            .handlers
            .into_iter()
            .fold(
                RpcBuilder::with_namespace(ctx.clone(), namespace),
                |rpc_builder, handler| (handler.register)(rpc_builder),
            )
            .consume();

        self.nested_routers
            .into_iter()
            .map(|(namespace, router)| router.build_rpc_module(ctx.clone(), Some(namespace)))
            .for_each(|router| {
                rpc_module.merge(router).unwrap();
            });

        rpc_module
    }

    // pub fn create_service(self, ctx: Ctx, stop_handle: StopHandle) -> ServerService<Ctx> {
    //     let svc_builder = jsonrpsee::server::Server::builder().to_service_builder();
    //
    //     // Create a top level module
    //     let rpc_module = self.build_rpc_module(ctx, None);
    //
    //     ServerService {
    //         rpc_module,
    //         stop_handle,
    //     }
    // }

    pub fn to_service(
        self,
        build_ctx: impl (Fn(&Request<Body>) -> Ctx) + Clone,
        stop_handle: StopHandle,
    ) -> impl Service<
        Request<Body>,
        Response = impl axum::response::IntoResponse,
        Error = Infallible,
        Future = impl Send,
    > + Clone {
        service_fn(move |req| {
            let ctx = build_ctx(&req);

            // WARN: Horrific amount of cloning
            let rpc_module = self.clone().build_rpc_module(ctx, None);

            let mut svc = jsonrpsee::server::Server::builder()
                .to_service_builder()
                .build(rpc_module.clone(), stop_handle.clone());

            async move {
                match svc.call(req).await {
                    Ok(v) => Ok::<_, Infallible>(v),
                    Err(_) => unreachable!(),
                }
            }
            .boxed()
        })
    }
}
