use std::{collections::HashSet, convert::Infallible, fmt::Write as _, fs, path::Path};

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
        Self::default()
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

    /// Write required bindings for this router the the provided directory. The directory will be
    /// cleared, so anything within will be lost.
    pub fn write_bindings_to_dir(&self, out_dir: impl AsRef<Path>) {
        let out_dir = out_dir.as_ref();

        // Make sure the directory path exists
        fs::create_dir_all(out_dir).unwrap();

        // Clear the directiry
        fs::remove_dir_all(out_dir).unwrap();

        // Re-create the directory
        fs::create_dir_all(out_dir).unwrap();

        // Export all the dependencies, and create their import statements
        let (imports, _types) = self
            .handlers
            .iter()
            .chain(
                self.nested_routers
                    .iter()
                    .flat_map(|(_, router)| &router.handlers),
            )
            .flat_map(|handler| {
                (handler.export_all_dependencies_to)(out_dir)
                    .unwrap()
                    .into_iter()
                    .map(|dep| {
                        (
                            format!("./{}", dep.output_path.to_str().unwrap()),
                            dep.ts_name,
                        )
                    })
                    .chain((handler.qubit_types)().into_iter().map(|ty| ty.to_ts()))
            })
            .fold(
                (String::new(), HashSet::new()),
                |(mut imports, mut types), ty| {
                    if types.contains(&ty) {
                        return (imports, types);
                    }

                    let (package, ty_name) = ty;

                    write!(
                        &mut imports,
                        r#"import type {{ {ty_name} }} from "{package}";"#,
                    )
                    .unwrap();

                    types.insert((package, ty_name));

                    (imports, types)
                },
            );

        // Generate server type
        let server_type = format!("export type QubitServer = {};", self.get_type());

        // Write out index file
        fs::write(out_dir.join("index.ts"), [imports, server_type].join("\n")).unwrap();
    }

    /// Turn the router into a [`tower::Service`], so that it can be nested into a HTTP server.
    ///
    /// Every incomming request has its own `Ctx` created for it, using the provided `build_ctx`
    /// method provided here. Generally this would involve cloning some pre-existing resources
    /// (database connections, channels, state), and capturing some information from the incoming
    /// [`Request`] included as a parameter.
    ///
    /// A closure can be provided to be called when the connection is closed, which will be
    /// provided with the `ctx` associated with that connection. For HTTP clients this isn't overly
    /// useful, but for WS clients this can be handy for tracking active clients.
    pub fn to_service<F, G>(
        self,
        build_ctx: impl (Fn(&Request<Body>) -> F) + Clone + Send + 'static,
        on_close: impl (Fn(Ctx) -> G) + Clone + Send + 'static,
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
        G: std::future::Future<Output = ()> + Send,
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
                let on_close = on_close.clone();

                async move {
                    let ctx = build_ctx(&req).await;

                    let rpc_module = s.build_rpc_module(ctx.clone(), None);

                    let mut svc = jsonrpsee::server::Server::builder()
                        .to_service_builder()
                        .build(rpc_module.clone(), stop_handle);

                    // Set up task to track when connection is closed
                    let on_session_closed = svc.on_session_closed();
                    tokio::spawn(async move {
                        // Wait for the session to close
                        on_session_closed.await;

                        // Run the on_close hook
                        on_close(ctx).await;
                    });

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

impl<Ctx> Default for Router<Ctx> {
    fn default() -> Self {
        Self {
            nested_routers: Default::default(),
            handlers: Default::default(),
        }
    }
}
