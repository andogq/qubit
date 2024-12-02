use std::collections::HashMap;
use std::{collections::HashSet, convert::Infallible, fmt::Write as _, fs, path::Path};

use axum::body::Body;
use futures::FutureExt;
use http::{HeaderValue, Method, Request};
use jsonrpsee::server::ws::is_upgrade_request;
pub use jsonrpsee::server::ServerHandle;
use jsonrpsee::RpcModule;
use tower::service_fn;
use tower::Service;
use tower::ServiceBuilder;

use crate::builder::*;
use crate::RequestKind;

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

        let header = String::from(include_str!("../header.txt"));

        // Export all the dependencies, and create their import statements
        let (imports, exports, _types) = self
            .get_handlers()
            .into_iter()
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
                (String::new(), String::new(), HashSet::new()),
                |(mut imports, mut exports, mut types), ty| {
                    if types.contains(&ty) {
                        return (imports, exports, types);
                    }

                    let (package, ty_name) = ty;

                    writeln!(
                        &mut imports,
                        r#"import type {{ {ty_name} }} from "{package}";"#,
                    )
                    .unwrap();

                    writeln!(
                        &mut exports,
                        r#"export type {{ {ty_name} }} from "{package}";"#,
                    )
                    .unwrap();

                    types.insert((package, ty_name));

                    (imports, exports, types)
                },
            );

        // Generate server type
        let server_type = format!("export type QubitServer = {};", self.get_type());

        // Write out index file
        fs::write(
            out_dir.join("index.ts"),
            [header, imports, exports, server_type]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
    }

    /// Turn the router into a [`tower::Service`], so that it can be nested into a HTTP server.
    /// The provided `ctx` will be cloned for each request.
    pub fn to_service(
        self,
        ctx: Ctx,
    ) -> (
        impl Service<
                hyper::Request<axum::body::Body>,
                Response = jsonrpsee::server::HttpResponse,
                Error = Infallible,
                Future = impl Send,
            > + Clone,
        ServerHandle,
    ) {
        // Generate the stop and server handles for the service
        let (stop_handle, server_handle) = jsonrpsee::server::stop_channel();

        // Build out the RPC module into a service
        let mut service = jsonrpsee::server::Server::builder()
            .set_http_middleware(ServiceBuilder::new().map_request(|mut req: Request<_>| {
                // Check if this is a GET request, and if it is convert it to a regular POST
                let request_type = if matches!(req.method(), &Method::GET)
                    && !is_upgrade_request(&req)
                {
                    // Change this request into a regular POST request, and indicate that it should
                    // be a query.
                    *req.method_mut() = Method::POST;

                    // Update the headers
                    let headers = req.headers_mut();
                    headers.insert(
                        hyper::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );
                    headers.insert(
                        hyper::header::ACCEPT,
                        HeaderValue::from_static("application/json"),
                    );

                    // Convert the `input` field of the query string into the request body
                    if let Some(body) = req
                        // Extract the query string
                        .uri()
                        .query()
                        // Parse the query string
                        .and_then(|query| serde_qs::from_str::<HashMap<String, String>>(query).ok())
                        // Take out the input
                        .and_then(|mut query| query.remove("input"))
                        // URL decode the input
                        .map(|input| urlencoding::decode(&input).unwrap_or_default().to_string())
                    {
                        // Set the request body
                        *req.body_mut() = Body::from(body);
                    }

                    RequestKind::Query
                } else {
                    RequestKind::Any
                };

                // Set the request kind
                req.extensions_mut().insert(request_type);

                req
            }))
            .to_service_builder()
            .build(self.build_rpc_module(ctx, None), stop_handle);

        (
            service_fn(move |req: hyper::Request<axum::body::Body>| {
                let call = service.call(req);

                async move {
                    match call.await {
                        Ok(response) => Ok::<_, Infallible>(response),
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
        let parent_namespace = namespace;
        self.nested_routers
            .into_iter()
            .fold(rpc_module, |mut rpc_module, (namespace, router)| {
                let namespace = if let Some(parent_namespace) = parent_namespace {
                    // WARN: Probably not great leaking here
                    format!("{parent_namespace}.{namespace}").leak()
                } else {
                    namespace
                };

                rpc_module
                    .merge(router.build_rpc_module(ctx.clone(), Some(namespace)))
                    .unwrap();

                rpc_module
            })
    }

    fn get_handlers(&self) -> Vec<HandlerCallbacks<Ctx>> {
        self.handlers
            .iter()
            .cloned()
            .chain(
                self.nested_routers
                    .iter()
                    .flat_map(|(_, router)| router.get_handlers()),
            )
            .collect()
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
