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

mod handler {
    use jsonrpsee::types::Params;
    use serde::Deserialize;
    use ts_rs::TS;

    use self::return_type::ReturnType;
    use super::ts_type::TsTypeTuple;

    /// A handler suitable for use with Qubit.
    ///
    /// The `Marker` generic is a utility in order to provide implementations for `Fn` traits which
    /// take generics as parameters.
    pub trait QubitHandler<MParams, MReturn>: 'static + Send + Sync + Clone {
        /// Context type this handler expects.
        type Ctx;

        type Params: TsTypeTuple;
        type Return: ReturnType<MReturn>;

        fn call(
            &self,
            ctx: &Self::Ctx,
            params: Params<'static>,
        ) -> impl Future<Output = Self::Return> + Send + Sync;
    }

    macro_rules! impl_handlers {
        (impl [$($ctx:ident, $($params:ident,)*)?]) => {
            impl<F, R, MReturn, $($ctx, $($params),*)?> QubitHandler<
                ($($ctx, $($params,)*)?), // MParams
                MReturn
            >
            for F
            where
                F: 'static + Send + Sync + Clone + Fn($(&$ctx, $($params),*)?) -> R,
                R: ReturnType<MReturn>,
                $(
                    $ctx: 'static + Send + Sync,
                    $($params: 'static + TS + Send + for<'a> Deserialize<'a>),*
                )?
            {
                type Ctx = impl_handlers!(ctx_ty [$($ctx)?]);

                type Params = ($($($params,)*)?);
                type Return = R;

                fn call(
                    &self,
                    #[allow(unused)] ctx: &Self::Ctx,
                    #[allow(unused)] params: Params<'static>
                ) -> impl Future<Output = Self::Return> + Send + Sync {
                    async move {
                        $(
                            #[allow(non_snake_case)]
                            let ($($params,)*) = match params.parse::<Self::Params>() {
                                Ok(params) => params,
                                Err(_e) => {
                                    // TODO: Something
                                    panic!("fukc");
                                }
                            };
                        )?

                        self($(ctx, $($params,)*)?)
                    }
                }
            }
        };

        (ctx_ty [$ctx:ty]) => {
            $ctx
        };
        (ctx_ty []) => {
            ()
        };

        (recurse []) => {};
        (recurse [$param:ident, $($params:ident,)*]) => {
            impl_handlers!($($params),*);
        };

        (count []) => { 0 };
        (count [$param:ident, $($params:ident,)*]) => {
            1 + impl_handlers!(count [$($params,)*])
        };

        ($($params:ident),* $(,)?) => {
            impl_handlers!(impl [$($params,)*]);
            impl_handlers!(recurse [$($params,)*]);
        };
    }

    impl_handlers!(
        P0, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11, P12, P13, P14, P15
    );

    #[derive(Clone, Debug)]
    pub enum HandlerKind {
        Query,
        Mutation,
        Subscription,
    }

    #[derive(Clone, Debug)]
    pub struct HandlerMeta {
        pub kind: HandlerKind,
        pub name: &'static str,
        pub param_names: &'static [&'static str],
    }

    #[derive(Clone)]
    pub struct HandlerDef<F> {
        pub handler: F,
        pub meta: HandlerMeta,
    }

    impl<F> std::ops::Deref for HandlerDef<F> {
        type Target = F;

        fn deref(&self) -> &Self::Target {
            &self.handler
        }
    }

    pub mod return_type {
        //! Handlers can return a wide range of different types, which may not be trivial to
        //! serialise or generate a TypeScript type for. The [`ReturnType`] trait allows for
        //! defining custom behaviour to take advantage of RPC functionality (such as streaming),
        //! or additional runtime logic in order to prepare a value for transmission.

        use std::{convert::Infallible, pin::pin};

        use futures::{Stream, StreamExt};
        use jsonrpsee::RpcModule;
        use serde::Serialize;
        use ts_rs::TS;

        use super::QubitHandler;

        /// Represents any type of value that may be returned from a handler. These may be Rust
        /// native types without a direct representation in TypeScript, and can use RPC-specific
        /// functionality in order to send the value (such as streaming). Therefore, `Repr` is the
        /// TypeScript-safe type which this value will be transformed to, and will be exposed in
        /// the generated types.
        ///
        /// The `Marker` allows this trait to be implemented on multiple traits. If there is a
        /// conflict in implementations, an error will be produced at the call site, rather than
        /// when implementing the trait.
        pub trait ReturnType<Marker>: 'static {
            /// Representation of the return value which will be serialised and sent to the client.
            type Repr: 'static + Clone + TS + Serialize + Send;

            fn register<Ctx, M2, RMarker>(
                module: &mut RpcModule<Ctx>,
                handler: impl QubitHandler<M2, RMarker, Ctx = Ctx, Return = Self>,
                method_name: String,
            ) where
                Ctx: 'static + Send + Sync;
        }

        /// Marker for any type that implements [`TS`]. This will directly produce the [`TsType`]
        /// as-is.
        #[doc(hidden)]
        pub struct TsMarker;
        impl<T> ReturnType<TsMarker> for T
        where
            T: 'static + Clone + TS + Serialize + Send,
        {
            type Repr = T;

            fn register<Ctx, M2, RMarker>(
                module: &mut RpcModule<Ctx>,
                handler: impl QubitHandler<M2, RMarker, Ctx = Ctx, Return = Self>,
                method_name: String,
            ) where
                Ctx: 'static + Send + Sync,
            {
                module
                    .register_async_method(
                        Box::leak(method_name.into_boxed_str()),
                        move |params, ctx, _extensions| {
                            let f = handler.clone();

                            async move { Ok::<_, Infallible>(f.call(&ctx, params).await) }
                        },
                    )
                    .unwrap();
            }
        }

        /// Marker for any type that is an iterator of [`TS`] items. The iterator will
        /// automatically be collected into a [`Vec`] before being returned.
        #[doc(hidden)]
        pub struct IterMarker;
        impl<T> ReturnType<IterMarker> for T
        where
            T: 'static + Iterator,
            T::Item: 'static + Clone + TS + Serialize + Send,
        {
            type Repr = Vec<T::Item>;

            fn register<Ctx, M2, RMarker>(
                module: &mut RpcModule<Ctx>,
                handler: impl QubitHandler<M2, RMarker, Ctx = Ctx, Return = Self>,
                method_name: String,
            ) where
                Ctx: 'static + Send + Sync,
            {
                module
                    .register_async_method(
                        Box::leak(method_name.into_boxed_str()),
                        move |params, ctx, _extensions| {
                            let f = handler.clone();

                            async move {
                                Ok::<_, Infallible>(f.call(&ctx, params).await.collect::<Vec<_>>())
                            }
                        },
                    )
                    .unwrap();
            }
        }

        /// Marker for a stream of [`TS`] items. Currently this just returns the [`TsType`] of the
        /// item, however it'd likely make more sense if it returned the `Subscription<...>` helper.
        #[doc(hidden)]
        pub struct StreamMarker;
        impl<T> ReturnType<StreamMarker> for T
        where
            T: 'static + Stream + Send,
            T::Item: 'static + Clone + TS + Serialize + Send,
        {
            // TODO: This should likely be a wrapper type of `Stream<T::Item>`, so that the types
            // can be correctly generated.
            type Repr = T::Item;

            fn register<Ctx, M2, RMarker>(
                module: &mut RpcModule<Ctx>,
                handler: impl QubitHandler<M2, RMarker, Ctx = Ctx, Return = Self>,
                method_name: String,
            ) where
                Ctx: 'static + Send + Sync,
            {
                let notif_method_name = format!("{method_name}_notif");
                let unsub_method_name = format!("{method_name}_unsub");

                module
                    .register_subscription(
                        Box::leak(method_name.into_boxed_str()),
                        Box::leak(notif_method_name.into_boxed_str()),
                        Box::leak(unsub_method_name.into_boxed_str()),
                        move |params, pending, ctx, _extensions| {
                            let f = handler.clone();

                            async move {
                                let sink = pending.accept().await.unwrap();

                                let mut stream = pin!(f.call(&ctx, params).await);

                                while let Some(item) = stream.next().await {
                                    let item = serde_json::value::to_raw_value(&item).unwrap();
                                    sink.send(item).await.unwrap();
                                }

                                Ok(())
                            }
                        },
                    )
                    .unwrap();
            }
        }

        #[cfg(test)]
        mod test {
            use super::*;

            mod register {
                use jsonrpsee::RpcModule;
                use serde::Deserialize;

                use super::*;

                /// Produce an iterator counting from 0 to 2 (inclusive).
                fn simple_iter() -> impl Iterator<Item = usize> {
                    0..3
                }

                /// Register a handler to a module, and return the module. The handler will be
                /// registered at `handler`.
                fn register_handler<T, M>(handler: fn() -> T) -> RpcModule<()>
                where
                    T: ReturnType<M>,
                {
                    let mut module = RpcModule::new(());
                    T::register(&mut module, handler, "handler".to_string());
                    module
                }

                /// Register a handler to a module, and call it, returning the value that was
                /// returned from the handler according to [`ReturnType`].
                fn test_handler<T, M>(handler: fn() -> T) -> T::Repr
                where
                    T: ReturnType<M>,
                    T::Repr: for<'a> Deserialize<'a>,
                {
                    let module = register_handler(handler);

                    let fut = module.call::<[(); 0], T::Repr>("handler", []);
                    futures::executor::block_on(fut).unwrap()
                }

                /// Primitive `TS` values should be returned as-is.
                #[test]
                fn ts() {
                    assert_eq!(test_handler(|| 123u32), 123);
                }

                /// Iterators should be collected and returned as a `Vec`.
                #[test]
                fn iter() {
                    assert_eq!(test_handler(simple_iter), vec![0, 1, 2]);
                }

                /// Stream should be consumed and each value returned one at a time.
                /// NOTE: Unfortunately, it's not possible to subscribe to a module when running outside of Tokio.
                #[tokio::test]
                async fn stream() {
                    let module = register_handler(|| futures::stream::iter(simple_iter()));
                    let mut subs = module.subscribe("handler", [] as [(); 0], 3).await.unwrap();

                    let mut next = async || subs.next::<usize>().await.unwrap().unwrap().0;

                    // Values should be produced in-order.
                    assert_eq!(0, next().await);
                    assert_eq!(1, next().await);
                    assert_eq!(2, next().await);

                    // Stream should be over, since there's no more items to be returned.
                    assert!(subs.next::<usize>().await.is_none());
                }
            }

            mod ts_type {
                use crate::ts::ts_type::TsType;

                use super::*;

                /// Helper to produce the [`TsType`] of the 'inner' value of a return type.
                fn ts_type_of_return_value<T: ReturnType<M>, M>(_v: T) -> TsType {
                    TsType::from_type::<T::Repr>()
                }

                #[test]
                fn ts() {
                    let ts_type = ts_type_of_return_value(1u32);
                    assert_eq!(ts_type.name, "number");
                    assert!(ts_type.is_primitive());
                }

                #[test]
                fn iter() {
                    let ts_type = ts_type_of_return_value(std::iter::once(true));
                    assert_eq!(ts_type.name, "Array<boolean>");
                    assert!(ts_type.is_primitive());
                }

                #[test]
                fn stream() {
                    let ts_type = ts_type_of_return_value(futures::stream::once(async { "hello" }));
                    assert_eq!(ts_type.name, "string");
                    assert!(ts_type.is_primitive());
                }
            }
        }
    }
}

use handler::*;

mod router {
    use crate::ts::handler::return_type::ReturnType;
    use jsonrpsee::RpcModule;

    use super::*;

    struct Router<Ctx> {
        nested_routers: Vec<(String, Router<Ctx>)>,
        register_methods: Vec<Box<dyn FnOnce(&mut RpcModule<Ctx>, Option<&str>)>>,
        handler_meta: Vec<HandlerMeta>,
    }

    impl<Ctx> Router<Ctx>
    where
        Ctx: 'static + Send + Sync,
    {
        /// Register the provided handler to this router.
        pub fn handler<F, M, RM>(mut self, handler: HandlerDef<F>) -> Self
        where
            F: QubitHandler<M, RM, Ctx = Ctx>,
        {
            self.register_methods.push(Box::new(|module, prefix| {
                let method_name = {
                    let handler_name = handler.meta.name.to_string();

                    if let Some(prefix) = prefix {
                        format!("{prefix}.{}", handler_name)
                    } else {
                        handler_name
                    }
                };

                F::Return::register(module, handler.handler, method_name);
            }));

            self.handler_meta.push(handler.meta);

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
            for register in self.register_methods {
                register(module, prefix);
            }

            // Add all nested routers.
            for (prefix, router) in self.nested_routers {
                router.add_to_module(module, Some(&prefix));
            }
        }
    }
}

#[cfg(test)]
mod test {
    use futures::{Stream, stream};

    use super::{
        handler::return_type::ReturnType,
        ts_type::{TsType, TsTypeTuple},
        *,
    };

    fn assert_handler<F, HandlerMarker, ReturnMarker>(
        _handler: F,
        _expected_ctx: F::Ctx,
    ) -> (Vec<TsType>, TsType)
    where
        F: QubitHandler<HandlerMarker, ReturnMarker>,
    {
        (
            F::Params::get_ts_types(),
            TsType::from_type::<<F::Return as ReturnType<_>>::Repr>(),
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
