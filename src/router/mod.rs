mod codegen;
mod rpc;

use std::{any::TypeId, collections::HashMap};

use lazy_static::lazy_static;
use linkme::distributed_slice;

use crate::{
    __private::HandlerMeta,
    FromRequestExtensions, RegisterableHandler,
    graph::Graph,
    handler::marker,
    router::{codegen::CodegenModule, rpc::RpcModule},
};

pub struct Router<Ctx> {
    handlers: Graph<String, Handler<Ctx>>,
}

struct Handler<Ctx> {
    rpc: <RpcModule<Ctx> as RouterModule<Ctx>>::Handler,
    codegen: <CodegenModule as RouterModule<Ctx>>::Handler,
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
        let handler_meta = HANDLER_DEFINITIONS_MAP.get(&TypeId::of::<F>()).unwrap();

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

    pub fn nest(mut self, prefix: impl ToString, other: Self) -> Self {
        let prefix = self.handlers.insert_prefix(None, prefix.to_string());
        self.handlers.nest(prefix, other.handlers);

        self
    }

    pub fn as_rpc(&self, ctx: Ctx) -> RpcModule<Ctx> {
        self.as_module(RpcModule::new(ctx), |handler| &handler.rpc)
    }

    pub fn as_codegen(&self) -> CodegenModule {
        self.as_module(CodegenModule::new(), |handler| &handler.codegen)
    }

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

trait RouterModule<Ctx> {
    type Handler: RouterModuleHandler<Ctx>;

    fn visit_handler(&mut self, path: &[&str], handler: &Self::Handler);
}

trait RouterModuleHandler<Ctx> {
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

    use std::any::Any;

    use crate::__private::HandlerKind;

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
