use std::marker::Send;

mod ts_type {
    //! Utilities for representing TypeScript types at runtime.

    use std::{any::TypeId, ops::Deref};

    use derive_more::Deref;
    use ts_rs::TS;

    /// Common components of [`TsType`].
    #[derive(Clone, Debug)]
    pub struct TsTypeCommon {
        /// TypeScript name of the type. Could be the primitive (`number`, `string`), or a
        /// user-defined type.
        pub name: String,
    }

    /// User-defined type.
    #[derive(Clone, Debug, Deref)]
    pub struct TsTypeUser {
        #[deref]
        common: TsTypeCommon,

        /// Rust type that this refers to. The same Rust type will correspond to the same
        /// TypeScript type (with the exception of numbers like [`i32`] and [`u32`] which are both
        ///  `number`).
        pub id: std::any::TypeId,
        /// Path that this user type will be exported to.
        pub output_path: std::path::PathBuf,
    }

    /// Type information to represent a type in TypeScript.
    #[derive(Clone, Debug)]
    pub enum TsType {
        /// Built-in TypeScript type.
        Primitive(TsTypeCommon),
        /// User-defined TypeScript type.
        User(TsTypeUser),
    }

    impl TsType {
        /// Determine if the type is primitive.
        pub fn is_primitive(&self) -> bool {
            matches!(self, Self::Primitive(_))
        }

        /// Determine if the type is user-defined.
        pub fn is_user(&self) -> bool {
            matches!(self, Self::User(_))
        }

        /// Produce type information for the given Rust type.
        pub fn from_type<T: 'static + TS + ?Sized>() -> Self {
            let common = TsTypeCommon { name: T::name() };

            // Determine whether the type is primitive or not based on whether the output path is defined.
            match T::output_path() {
                Some(output_path) => Self::User(TsTypeUser {
                    common,
                    id: TypeId::of::<T>(),
                    output_path,
                }),
                None => Self::Primitive(common),
            }
        }
    }

    impl Deref for TsType {
        type Target = TsTypeCommon;

        fn deref(&self) -> &Self::Target {
            match self {
                TsType::Primitive(ts_type_common) => ts_type_common,
                TsType::User(ts_type_user) => ts_type_user,
            }
        }
    }

    /// Tuple of [`TsType`] types.
    pub trait TsTypeTuple {
        /// Produce all of the [`TsType`] for each of the types in the tuple, in order.
        fn get_ts_types() -> Vec<TsType>;
    }

    macro_rules! impl_ts_type_tuple {
        (impl [$($params:ident,)*]) => {
            impl<$($params: 'static + TS,)*> TsTypeTuple for ($($params,)*) {
                fn get_ts_types() -> Vec<TsType> {
                    vec![$(TsType::from_type::<$params>(),)*]
                }
            }
        };

        (recurse []) => {};

        (recurse [$param:ident, $($params:ident,)*]) => {
            impl_ts_type_tuple!($($params),*);
        };

        ($($params:ident),* $(,)?) => {
            impl_ts_type_tuple!(impl [$($params,)*]);
            impl_ts_type_tuple!(recurse [$($params,)*]);
        };
    }

    impl_ts_type_tuple!(
        T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
    );

    #[cfg(test)]
    mod test {
        use super::*;

        mod ts_type {
            use super::*;

            #[test]
            fn valid_primitive() {
                let ts_type = TsType::from_type::<u32>();
                assert_eq!(ts_type.name, "number");
                assert!(ts_type.is_primitive());
            }

            #[test]
            fn valid_user_defined() {
                #[derive(TS)]
                struct MyType;

                let ts_type = TsType::from_type::<MyType>();
                assert_eq!(ts_type.name, "MyType");
                assert!(ts_type.is_user());
            }

            mod ts_tupe_tuple {
                use super::*;

                #[test]
                fn empty() {
                    let types = <()>::get_ts_types();
                    assert!(types.is_empty());
                }

                #[test]
                fn single() {
                    let types = <(u32,)>::get_ts_types();
                    assert_eq!(types.len(), 1);
                    assert_eq!(types[0].name, "number");
                }

                #[test]
                fn multiple() {
                    let types = <(u32, bool, String)>::get_ts_types();
                    assert_eq!(types.len(), 3);
                    assert_eq!(types[0].name, "number");
                    assert_eq!(types[1].name, "boolean");
                    assert_eq!(types[2].name, "string");
                }
            }
        }
    }
}

mod handler;
use handler::*;

mod router {
    use jsonrpsee::RpcModule;

    use super::{handler::reflection::*, *};

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
        pub fn handler<
            F,
            MSig,
            MValue: marker::ResponseMarker,
            MReturn: marker::HandlerReturnMarker,
        >(
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
}

#[cfg(test)]
mod test {
    use futures::{Stream, stream};

    use super::{
        handler::response::ResponseValue,
        ts_type::{TsType, TsTypeTuple},
        *,
    };

    fn assert_handler<F, MSig, MValue, MReturn>(
        _handler: F,
        _expected_ctx: F::Ctx,
    ) -> (Vec<TsType>, TsType)
    where
        MValue: marker::ResponseMarker,
        MReturn: marker::HandlerReturnMarker,
        F: RegisterableHandler<MSig, MValue, MReturn>,
    {
        (
            F::Params::get_ts_types(),
            TsType::from_type::<<F::Response as ResponseValue<_>>::Value>(),
        )
    }

    #[test]
    fn unit_handler() {
        fn handler() {}

        let (param_tys, return_ty) = assert_handler(handler, ());
        assert!(param_tys.is_empty());
        assert_eq!(return_ty.name, "null");
    }

    #[test]
    fn single_ctx_param() {
        struct Ctx;
        fn handler(_ctx: &Ctx) {}

        let (param_tys, return_ty) = assert_handler(handler, Ctx);
        assert!(param_tys.is_empty());
        assert_eq!(return_ty.name, "null");
    }

    #[test]
    fn only_return_ty() {
        fn handler() -> bool {
            todo!()
        }

        let (param_tys, return_ty) = assert_handler(handler, ());
        assert!(param_tys.is_empty());
        assert_eq!(return_ty.name, "boolean");
    }

    #[test]
    fn ctx_and_param() {
        struct Ctx;
        fn handler(_ctx: &Ctx, _a: u32) {}

        let (param_tys, return_ty) = assert_handler(handler, Ctx);
        assert_eq!(param_tys.len(), 1);
        assert_eq!(param_tys[0].name, "number");
        assert_eq!(return_ty.name, "null");
    }

    #[test]
    fn ctx_and_param_and_return() {
        struct Ctx;
        fn handler(_ctx: &Ctx, _a: u32) -> bool {
            todo!()
        }

        let (param_tys, return_ty) = assert_handler(handler, Ctx);
        assert_eq!(param_tys.len(), 1);
        assert_eq!(param_tys[0].name, "number");
        assert_eq!(return_ty.name, "boolean");
    }

    #[test]
    fn ctx_and_multi_param() {
        struct Ctx;
        fn handler(_ctx: &Ctx, _a: u32, _b: String, _c: bool) {
            todo!()
        }

        let (param_tys, return_ty) = assert_handler(handler, Ctx);
        assert_eq!(param_tys.len(), 3);
        assert_eq!(param_tys[0].name, "number");
        assert_eq!(param_tys[1].name, "string");
        assert_eq!(param_tys[2].name, "boolean");
        assert_eq!(return_ty.name, "null");
    }

    #[test]
    fn ctx_and_multi_param_and_return() {
        struct Ctx;
        fn handler(_ctx: &Ctx, _a: u32, _b: String, _c: bool) -> bool {
            todo!()
        }

        let (param_tys, return_ty) = assert_handler(handler, Ctx);
        assert_eq!(param_tys.len(), 3);
        assert_eq!(param_tys[0].name, "number");
        assert_eq!(param_tys[1].name, "string");
        assert_eq!(param_tys[2].name, "boolean");
        assert_eq!(return_ty.name, "boolean");
    }

    #[test]
    fn produce_iter() {
        fn handler() -> impl Iterator<Item = u32> {
            [1, 2, 3].into_iter()
        }

        let (param_tys, return_ty) = assert_handler(handler, ());
        assert!(param_tys.is_empty());
        assert_eq!(return_ty.name, "Array<number>");
    }

    #[test]
    fn produce_stream() {
        fn handler() -> impl Stream<Item = u32> {
            stream::iter([1, 2, 3])
        }

        let (param_tys, return_ty) = assert_handler(handler, ());
        assert!(param_tys.is_empty());
        assert_eq!(return_ty.name, "number");
    }
}
