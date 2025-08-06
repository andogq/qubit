use std::{
    any::{Any, TypeId},
    collections::HashMap,
    convert::Infallible,
    fs::OpenOptions,
    io::Write,
    path::Path,
};

use axum::response::IntoResponse;
use futures::FutureExt;
use http::{HeaderValue, Method, Request, header};
use jsonrpsee::{
    RpcModule,
    server::{Server, ServerHandle, stop_channel, ws::is_upgrade_request},
};
use lazy_static::lazy_static;
use linkme::distributed_slice;
use tower::{Service, ServiceBuilder, service_fn};

use crate::{
    FromRequestExtensions,
    codegen::{Backend, Codegen},
};

use super::{
    handler::{RegisterableHandler, marker, reflection::*},
    ts::TsRouter,
};

/// A closure which will register a handler to the provided [`RpcModule`], with an optional
/// prefix. The registration is guarenteed to only take place once, so the closure is free to
/// move values without cloning.
type HandlerRegistration<Ctx> = Box<dyn FnOnce(&mut RpcModule<Ctx>, Option<&str>)>;

/// Collection of handlers and nested routers, which combine to create an RPC API, including
/// TypeScript bindings.
pub struct Router<Ctx> {
    /// Registration methods for all handlers present in this router.
    handler_registrations: Vec<(Option<String>, HandlerRegistration<Ctx>)>,
    /// Type information for generating TypeScript type for the router.
    ts_router: TsRouter,
}

impl<Ctx> Router<Ctx> {
    /// Create an empty router.
    pub fn new() -> Self {
        Router {
            handler_registrations: Vec::new(),
            ts_router: TsRouter::new(),
        }
    }
}

#[distributed_slice]
pub static HANDLER_DEFINITIONS: [fn() -> (TypeId, HandlerMeta)];

lazy_static! {
    static ref HANDLER_DEFINITIONS_MAP: HashMap<TypeId, HandlerMeta> = HANDLER_DEFINITIONS
        .into_iter()
        .map(|def_fn| def_fn())
        .collect();
}

impl<Ctx> Router<Ctx>
where
    Ctx: 'static + Clone + Send + Sync,
{
    /// Register the provided handler to this router.
    pub fn handler<F, MSig, MValue: marker::ResponseMarker, MReturn: marker::HandlerReturnMarker>(
        mut self,
        handler: F,
    ) -> Self
    where
        F: RegisterableHandler<Ctx, MSig, MValue, MReturn>,
        F::Ctx: FromRequestExtensions<Ctx>,
    {
        let handler_meta = HANDLER_DEFINITIONS_MAP.get(&handler.type_id()).unwrap();
        self.ts_router.add_handler(handler_meta, &handler);

        // Create the registration function for this handler.
        self.handler_registrations.push((
            None,
            Box::new(|module, prefix| {
                // Build the method name, depending if there's a prefix or not.
                let method_name = {
                    let handler_name = handler_meta.name.to_string();

                    if let Some(prefix) = prefix {
                        format!("{prefix}.{handler_name}")
                    } else {
                        handler_name
                    }
                };

                // Use the registration method derived from the `ReturnType` of this handler.
                handler.register(module, method_name);
            }),
        ));

        self
    }

    /// Nest a router at the provided prefix.
    pub fn nest(mut self, prefix: impl AsRef<str>, router: Router<Ctx>) -> Self {
        let prefix = prefix.as_ref();

        self.handler_registrations
            .extend(router.handler_registrations.into_iter().map(
                |(handler_prefix, registration)| {
                    (
                        Some(match handler_prefix {
                            Some(handler_prefix) => format!("{prefix}.{handler_prefix}"),
                            None => prefix.to_string(),
                        }),
                        registration,
                    )
                },
            ));
        self.ts_router.nest(prefix, router.ts_router);

        self
    }

    /// Generate the TypeScript for this router, and write it to the provided path.
    pub fn generate_type(&self, output_path: impl AsRef<Path>) -> std::io::Result<()> {
        const QUBIT_HEADER: &str = include_str!("./header.txt");

        let router_typescript = self.generate_type_to_string();

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_path.as_ref())
            .unwrap();

        writeln!(file, "{QUBIT_HEADER}").unwrap();

        // TODO: Do this else where.
        writeln!(
            file,
            r#"import type {{ Query, Mutation, Subscription }} from "@qubit-rs/client";"#
        )
        .unwrap();

        writeln!(file, "{router_typescript}").unwrap();

        Ok(())
    }

    pub fn generate_type_to_string(&self) -> String {
        self.ts_router.generate_typescript()
    }

    /// Consume this router, and produce an [`RpcModule`].
    pub fn into_module(self, ctx: Ctx) -> RpcModule<Ctx> {
        self.handler_registrations.into_iter().fold(
            RpcModule::new(ctx),
            |mut module, (prefix, register)| {
                register(&mut module, prefix.as_deref());
                module
            },
        )
    }

    pub fn into_service(
        self,
        ctx: Ctx,
    ) -> (
        impl Service<
            Request<axum::body::Body>,
            Error = Infallible,
            Future = impl Send,
            Response = impl IntoResponse,
        > + Clone,
        ServerHandle,
    ) {
        let module = self.into_module(ctx);
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

impl<Ctx> Default for Router<Ctx> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use serde::Deserialize;

    use super::*;

    async fn run_handler<T>(module: &RpcModule<()>, method: &str) -> T
    where
        T: Clone + for<'a> Deserialize<'a>,
    {
        module.call(method, [] as [(); 0]).await.unwrap()
    }

    #[test]
    fn empty_router() {
        let router = Router::new();
        let module = router.into_module(());
        // No methods should be present.
        assert_eq!(module.method_names().count(), 0);
    }

    macro_rules! define_handler {
        (|| $body:expr, $meta:expr $(,)?) => {{
            fn handler() -> u32 {
                $body
            }

            #[distributed_slice(HANDLER_DEFINITIONS)]
            static DEF: fn() -> (TypeId, HandlerMeta) = || (Any::type_id(&handler), $meta);
            handler
        }};
    }

    #[tokio::test]
    async fn single_handler() {
        let module = Router::new()
            .handler(define_handler! {
                || 123u32,
                HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "handler",
                    param_names: &[],
                },
            })
            .into_module(());

        assert_eq!(module.method_names().count(), 1);
        assert_eq!(run_handler::<u32>(&module, "handler").await, 123);
    }

    #[tokio::test]
    async fn multiple_handlers() {
        let module = Router::new()
            .handler(define_handler! {
                || 123u32,
                HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "handler_1",
                    param_names: &[],
                },
            })
            .handler(define_handler! {
                || 321u32,
                HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "handler_2",
                    param_names: &[],
                },
            })
            .into_module(());

        assert_eq!(module.method_names().count(), 2);
        assert_eq!(run_handler::<u32>(&module, "handler_1").await, 123);
        assert_eq!(run_handler::<u32>(&module, "handler_2").await, 321);
    }

    #[tokio::test]
    async fn nested_router() {
        let module = Router::new()
            .nest(
                "nested",
                Router::new().handler(define_handler! {
                    || 123u32,
                    HandlerMeta {
                        kind: HandlerKind::Query,
                        name: "handler",
                        param_names: &[],
                    },
                }),
            )
            .into_module(());

        assert_eq!(module.method_names().count(), 1);
        assert_eq!(run_handler::<u32>(&module, "nested.handler").await, 123);
    }

    #[tokio::test]
    async fn multiple_nested_router() {
        let module = Router::new()
            .nest(
                "nested_1",
                Router::new().handler(define_handler! {
                    || 123u32,
                    HandlerMeta {
                        kind: HandlerKind::Query,
                        name: "handler",
                        param_names: &[],
                    },
                }),
            )
            .nest(
                "nested_2",
                Router::new().handler(define_handler! {
                    || 321u32,
                    HandlerMeta {
                        kind: HandlerKind::Query,
                        name: "handler",
                        param_names: &[],
                    },
                }),
            )
            .into_module(());

        assert_eq!(module.method_names().count(), 2);
        assert_eq!(run_handler::<u32>(&module, "nested_1.handler").await, 123);
        assert_eq!(run_handler::<u32>(&module, "nested_2.handler").await, 321);
    }

    #[tokio::test]
    async fn everything() {
        let module = Router::new()
            .handler(define_handler! {
                || 123u32,
                HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "handler_1",
                    param_names: &[],
                },
            })
            .handler(define_handler! {
                || 321u32,
                HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "handler_2",
                    param_names: &[],
                },
            })
            .nest(
                "nested_1",
                Router::new().handler(define_handler! {
                    || 456u32,
                    HandlerMeta {
                        kind: HandlerKind::Query,
                        name: "handler",
                        param_names: &[],
                    },
                }),
            )
            .nest(
                "nested_2",
                Router::new().handler(define_handler! {
                    || 654u32,
                    HandlerMeta {
                        kind: HandlerKind::Query,
                        name: "handler",
                        param_names: &[],
                    },
                }),
            )
            .into_module(());

        assert_eq!(module.method_names().count(), 4);
        assert_eq!(run_handler::<u32>(&module, "handler_1").await, 123);
        assert_eq!(run_handler::<u32>(&module, "handler_2").await, 321);
        assert_eq!(run_handler::<u32>(&module, "nested_1.handler").await, 456);
        assert_eq!(run_handler::<u32>(&module, "nested_2.handler").await, 654);
    }
}
