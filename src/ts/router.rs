use jsonrpsee::RpcModule;

use super::handler::{RegisterableHandler, marker, reflection::*};

/// A closure which will register a handler to the provided [`RpcModule`], with an optional
/// prefix. The registration is guarenteed to only take place once, so the closure is free to
/// move values without cloning.
type HandlerRegistration<Ctx> = Box<dyn FnOnce(&mut RpcModule<Ctx>, Option<&str>)>;

/// Collection of handlers and nested routers, which combine to create an RPC API, including
/// TypeScript bindings.
struct Router<Ctx> {
    /// Routers nested within this router, and the prefix they are located.
    nested_routers: Vec<(String, Router<Ctx>)>,
    /// Registration methods for all handlers present in this router.
    handler_registrations: Vec<HandlerRegistration<Ctx>>,
    /// [`HandlerMeta`] for all of the handlers registered to this router.
    handler_meta: Vec<HandlerMeta>,
}

impl<Ctx> Router<Ctx> {
    /// Create an empty router.
    pub fn new() -> Self {
        Router {
            nested_routers: Vec::new(),
            handler_registrations: Vec::new(),
            handler_meta: Vec::new(),
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
        // Create the registration function for this handler.
        self.handler_registrations.push(Box::new(|module, prefix| {
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
        }));

        self.handler_meta.push(handler.meta);

        self
    }

    /// Nest a router at the provided prefix.
    pub fn nest(mut self, prefix: impl ToString, router: Router<Ctx>) -> Self {
        self.nested_routers.push((prefix.to_string(), router));

        self
    }

    /// Consume this router, and produce an [`RpcModule`].
    pub fn into_module(self, ctx: Ctx) -> RpcModule<Ctx> {
        let mut module = RpcModule::new(ctx);
        self.add_to_module(&mut module, None);
        module
    }

    /// Consume this router, adding it to the provided [`RpcModule`].
    fn add_to_module(self, module: &mut RpcModule<Ctx>, prefix: Option<&str>) {
        // Add the handlers for this router.
        for register in self.handler_registrations {
            register(module, prefix);
        }

        // Add all nested routers.
        for (prefix, router) in self.nested_routers {
            router.add_to_module(module, Some(&prefix));
        }
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
