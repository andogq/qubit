//! The [`Router`] is the key to the exposed API of Qubit. It provides the core of the hierarchy
//! structure, but delegates any actual work (codegen, RPC integration) to [`RpcModule`]s.

mod codegen;
mod rpc;

use crate::{
    FromRequestExtensions, RegisterableHandler,
    handler::marker,
    reflection::handler::HandlerMeta,
    router::{codegen::CodegenModule, rpc::RpcModule},
    util::Graph,
};

/// Qubit router, which will contain all handlers.
pub struct Router<Ctx> {
    handlers: Graph<String, Handler<Ctx>>,
}

/// Actual information stored for each handler added to the router. Each [`RpcModule`] will have
/// its own handler representation, used to type-erase the actual handler.
struct Handler<Ctx> {
    rpc: <RpcModule<Ctx> as RouterModule<Ctx>>::Handler,
    codegen: <CodegenModule as RouterModule<Ctx>>::Handler,
}

impl<Ctx> Router<Ctx>
where
    Ctx: 'static + Clone + Send + Sync,
{
    /// Create a new router.
    pub fn new() -> Self {
        Self {
            handlers: Graph::new(),
        }
    }

    /// Register the provided handler to this router.
    pub fn handler<F, MSig, MValue: marker::ResponseMarker, MReturn: marker::HandlerReturnMarker>(
        mut self,
        handler: F,
    ) -> Self
    where
        F: RegisterableHandler<Ctx, MSig, MValue, MReturn>,
        F::Ctx: FromRequestExtensions<Ctx>,
    {
        let handler_meta = HandlerMeta::of(&handler);

        // Insert a prefix corresponding with the handler's name.
        let prefix = self
            .handlers
            .insert_prefix(None, handler_meta.name.to_string());

        // Insert the handler.
        self.handlers.insert_item(
            prefix,
            Handler {
                codegen: <CodegenModule as RouterModule<Ctx>>::Handler::from_handler(
                    handler.clone(),
                    handler_meta,
                ),
                rpc: <RpcModule<Ctx> as RouterModule<Ctx>>::Handler::from_handler(
                    handler.clone(),
                    handler_meta,
                ),
            },
        );

        self
    }

    /// Nest another router at the provided prefix.
    pub fn nest(mut self, prefix: impl ToString, other: Self) -> Self {
        let prefix = self.handlers.insert_prefix(None, prefix.to_string());
        self.handlers.nest(prefix, other.handlers);

        self
    }

    /// Build an [`RpcModule`] from this router. This is required in order to start the RPC server.
    pub fn as_rpc(&self, ctx: Ctx) -> RpcModule<Ctx> {
        self.as_module(RpcModule::new(ctx), |handler| &handler.rpc)
    }

    /// Build a [`CodegenModule`] From this router. This is required to generate types for the
    /// server.
    pub fn as_codegen(&self) -> CodegenModule {
        self.as_module(CodegenModule::new(), |handler| &handler.codegen)
    }

    /// Helper to convert this router into the provided [`RouterModule`].
    fn as_module<M: RouterModule<Ctx>>(
        &self,
        module: M,
        handler_map: impl Fn(&Handler<Ctx>) -> &M::Handler,
    ) -> M {
        self.handlers
            .iter()
            .fold(module, |mut module, (path, handler)| {
                module.visit_handler(
                    &path.into_iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    handler_map(handler),
                );

                module
            })
    }
}

impl<Ctx> Default for Router<Ctx>
where
    Ctx: 'static + Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Common functionality exposed by router modules. The module will be provided a handler, which it
/// must generate a type-erased [`RouterModule::Handler`] representation from. At a later point,
/// [`RouterModule::visit_handler`] will be repeatedly called with each [`RouterModule::Handler`]
/// and the path which it was present.
trait RouterModule<Ctx> {
    /// Type-erased representation of a handler.
    type Handler: RouterModuleHandler<Ctx>;

    /// Will be called once for each handler present, along with the path that it resides at.
    fn visit_handler(&mut self, path: &[&str], handler: &Self::Handler);
}

/// Converts a handler existing as a generic (`F`) into a type-erased value. Intended for use with
/// the [`RouterModule`] trait.
trait RouterModuleHandler<Ctx> {
    /// Produce a type-erased value from the provided handler, and the associated metadata for the
    /// handler.
    fn from_handler<F, MSig, MValue: marker::ResponseMarker, MReturn: marker::HandlerReturnMarker>(
        handler: F,
        meta: &'static HandlerMeta,
    ) -> Self
    where
        F: RegisterableHandler<Ctx, MSig, MValue, MReturn>,
        F::Ctx: FromRequestExtensions<Ctx>;
}

#[cfg(test)]
mod test {
    use jsonrpsee::RpcModule;
    use serde::Deserialize;

    use std::any::{Any, TypeId};

    use crate::reflection::handler::{HANDLER_DEFINITIONS, HandlerKind};

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
        let module = router.as_rpc(()).into_module();
        // No methods should be present.
        assert_eq!(module.method_names().count(), 0);
    }

    /// Manually register the provided handler, with the associated [`HandlerMeta`]. This will
    /// normally be done with the [`crate::handler`] proc-macro.
    macro_rules! define_handler {
        (|| $body:expr, $meta:expr $(,)?) => {{
            fn handler() -> u32 {
                $body
            }

            #[linkme::distributed_slice(HANDLER_DEFINITIONS)]
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
            .as_rpc(())
            .into_module();

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
            .as_rpc(())
            .into_module();

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
            .as_rpc(())
            .into_module();

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
            .as_rpc(())
            .into_module();

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
            .as_rpc(())
            .into_module();

        assert_eq!(module.method_names().count(), 4);
        assert_eq!(run_handler::<u32>(&module, "handler_1").await, 123);
        assert_eq!(run_handler::<u32>(&module, "handler_2").await, 321);
        assert_eq!(run_handler::<u32>(&module, "nested_1.handler").await, 456);
        assert_eq!(run_handler::<u32>(&module, "nested_2.handler").await, 654);
    }
}
