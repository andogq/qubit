use std::{convert::Infallible, fs, path::Path};

use futures::FutureExt;
use http::Request;
use hyper::{service::service_fn, Body};
pub use jsonrpsee::server::ServerHandle;
use jsonrpsee::RpcModule;
use tower::Service;

use crate::builder::*;

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
    /// Create a new instance of the router.
    pub fn new() -> Self {
        Self {
            nested_routers: Vec::new(),
            handlers: Vec::new(),
        }
    }

    /// Attach a handler to the router.
    pub fn handler<H: Handler<Ctx>>(mut self, handler: H) -> Self {
        self.handlers.push(HandlerCallbacks::from_handler(handler));

        self
    }

    /// Nest another router within this router, under the provided namespace.
    pub fn nest(mut self, namespace: &'static str, router: Router<Ctx>) -> Self {
        self.nested_routers.push((namespace, router));

        self
    }

    /// Write this router's type to the provided path, often a path that is reachable from the
    /// TypeScript client.
    pub fn write_type_to_file(&self, path: impl AsRef<Path>) {
        // Generate all dependencies for this router
        let dependencies = {
            let mut registry = TypeRegistry::default();
            self.add_dependencies(&mut registry);
            registry
        };

        // Generate the type for this router
        let router = format!("export type QubitServer = {};", self.get_type());

        // Build the file contents
        let content = format!("{dependencies}\n{router}");

        // Write out
        fs::write(path, content).unwrap();
    }

    /// Turn the router into a [`tower::Service`], so that it can be nested into a HTTP server.
    ///
    /// Every incomming request has its own `Ctx` created for it, using the provided `build_ctx`
    /// method provided here. Generally this would involve cloning some pre-existing resources
    /// (database connections, channels, state), and capturing some information from the incomming
    /// [`Request`] included as a parameter.
    pub fn to_service<F>(
        self,
        build_ctx: impl (Fn(&Request<Body>) -> F) + Clone + Send + 'static,
    ) -> (
        impl Service<
                Request<Body>,
                Response = impl axum::response::IntoResponse,
                Error = Infallible,
                Future = impl Send,
            > + Clone,
        ServerHandle,
    )
    where
        F: std::future::Future<Output = Ctx> + Send,
    {
        // Generate the stop and server handles for the service
        let (stop_handle, server_handle) = jsonrpsee::server::stop_channel();

        (
            service_fn(move |req| {
                let stop_handle = stop_handle.clone();

                // WARN: Horrific amount of cloning, required as it is not possible to swap out the
                // context on a pre-exising RpcModule.
                let s = self.clone();

                let build_ctx = build_ctx.clone();

                async move {
                    let ctx = build_ctx(&req).await;

                    let rpc_module = s.build_rpc_module(ctx, None);

                    let mut svc = jsonrpsee::server::Server::builder()
                        .to_service_builder()
                        .build(rpc_module.clone(), stop_handle);

                    match svc.call(req).await {
                        Ok(v) => Ok::<_, Infallible>(v),
                        Err(_) => unreachable!(),
                    }
                }
                .boxed()
            }),
            server_handle,
        )
    }

    /// Adds all of the dependencies for this router to the provided dependency list.
    fn add_dependencies(&self, dependencies: &mut TypeRegistry) {
        // Add all handler dependencies
        self.handlers
            .iter()
            .for_each(|handler| (handler.export_types)(dependencies));

        // Add dependencies for nested routers
        self.nested_routers
            .iter()
            .for_each(|(_, router)| router.add_dependencies(dependencies));
    }

    /// Get the TypeScript type of this router.
    fn get_type(&self) -> String {
        // Generate types of all handlers, including nested handlers
        let handlers = self
            .handlers
            .iter()
            // Generate types of handlers
            .map(|handler| {
                let handler_type = (handler.get_type)();
                format!("{}: {}", handler_type.name, handler_type.signature)
            })
            .chain(
                // Generate types of nested routers
                self.nested_routers.iter().map(|(namespace, router)| {
                    let router_type = router.get_type();
                    format!("{namespace}: {router_type}")
                }),
            )
            .collect::<Vec<_>>();

        // Generate the router type
        format!("{{ {} }}", handlers.join(", "))
    }

    /// Generate a [`jsonrpsee::RpcModule`] from this router, with an optional namespace.
    ///
    /// Uses an [`RpcBuilder`] to incrementally add query and subcription handlers, passing the
    /// instance through to the [`HandlerCallbacks`] attached to this router, so they can register
    /// against the [`RpcModule`] (including namespacing).
    fn build_rpc_module(self, ctx: Ctx, namespace: Option<&'static str>) -> RpcModule<Ctx> {
        let rpc_module = self
            .handlers
            .into_iter()
            .fold(
                RpcBuilder::with_namespace(ctx.clone(), namespace),
                |rpc_builder, handler| (handler.register)(rpc_builder),
            )
            .build();

        // Generate modules for nested routers, and merge them with the existing router
        self.nested_routers
            .into_iter()
            .fold(rpc_module, |mut rpc_module, (namespace, router)| {
                rpc_module
                    .merge(router.build_rpc_module(ctx.clone(), Some(namespace)))
                    .unwrap();

                rpc_module
            })
    }
}
