use std::path::Path;

use jsonrpsee::RpcModule;

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
struct Router<Ctx> {
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

impl<Ctx> Router<Ctx>
where
    Ctx: 'static + Send + Sync,
{
    /// Register the provided handler to this router.
    pub fn handler<F, MSig, MValue: marker::ResponseMarker, MReturn: marker::HandlerReturnMarker>(
        mut self,
        handler: HandlerDef<F>,
    ) -> Self
    where
        F: RegisterableHandler<MSig, MValue, MReturn, Ctx = Ctx>,
    {
        self.ts_router.add_handler(
            handler.meta.name,
            handler.meta.param_names,
            &handler.handler,
        );

        // Create the registration function for this handler.
        self.handler_registrations.push((
            None,
            Box::new(|module, prefix| {
                // Build the method name, depending if there's a prefix or not.
                let method_name = {
                    let handler_name = handler.meta.name.to_string();

                    if let Some(prefix) = prefix {
                        format!("{prefix}.{}", handler_name)
                    } else {
                        handler_name
                    }
                };

                // Use the registration method derived from the `ReturnType` of this handler.
                handler.handler.register(module, method_name);
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
        let router_typescript = self.ts_router.generate_typescript();
        std::fs::write(output_path.as_ref(), router_typescript)?;
        Ok(())
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

    #[tokio::test]
    async fn single_handler() {
        let module = Router::new()
            .handler(HandlerDef {
                handler: || 123u32,
                meta: HandlerMeta {
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
            .handler(HandlerDef {
                handler: || 123u32,
                meta: HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "handler_1",
                    param_names: &[],
                },
            })
            .handler(HandlerDef {
                handler: || "hello",
                meta: HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "handler_2",
                    param_names: &[],
                },
            })
            .into_module(());

        assert_eq!(module.method_names().count(), 2);
        assert_eq!(run_handler::<u32>(&module, "handler_1").await, 123);
        assert_eq!(run_handler::<String>(&module, "handler_2").await, "hello");
    }

    #[tokio::test]
    async fn nested_router() {
        let module = Router::new()
            .nest(
                "nested",
                Router::new().handler(HandlerDef {
                    handler: || 123u32,
                    meta: HandlerMeta {
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
                Router::new().handler(HandlerDef {
                    handler: || 123u32,
                    meta: HandlerMeta {
                        kind: HandlerKind::Query,
                        name: "handler",
                        param_names: &[],
                    },
                }),
            )
            .nest(
                "nested_2",
                Router::new().handler(HandlerDef {
                    handler: || "hello",
                    meta: HandlerMeta {
                        kind: HandlerKind::Query,
                        name: "handler",
                        param_names: &[],
                    },
                }),
            )
            .into_module(());

        assert_eq!(module.method_names().count(), 2);
        assert_eq!(run_handler::<u32>(&module, "nested_1.handler").await, 123);
        assert_eq!(
            run_handler::<String>(&module, "nested_2.handler").await,
            "hello"
        );
    }

    #[tokio::test]
    async fn everything() {
        let module = Router::new()
            .handler(HandlerDef {
                handler: || 123u32,
                meta: HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "handler_1",
                    param_names: &[],
                },
            })
            .handler(HandlerDef {
                handler: || "hello",
                meta: HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "handler_2",
                    param_names: &[],
                },
            })
            .nest(
                "nested_1",
                Router::new().handler(HandlerDef {
                    handler: || 456u32,
                    meta: HandlerMeta {
                        kind: HandlerKind::Query,
                        name: "handler",
                        param_names: &[],
                    },
                }),
            )
            .nest(
                "nested_2",
                Router::new().handler(HandlerDef {
                    handler: || "world",
                    meta: HandlerMeta {
                        kind: HandlerKind::Query,
                        name: "handler",
                        param_names: &[],
                    },
                }),
            )
            .into_module(());

        assert_eq!(module.method_names().count(), 4);
        assert_eq!(run_handler::<u32>(&module, "handler_1").await, 123);
        assert_eq!(run_handler::<String>(&module, "handler_2").await, "hello");
        assert_eq!(run_handler::<u32>(&module, "nested_1.handler").await, 456);
        assert_eq!(
            run_handler::<String>(&module, "nested_2.handler").await,
            "world"
        );
    }
}
