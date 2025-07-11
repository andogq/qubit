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

    use super::ts_type::TsTypeTuple;

    /// A handler suitable for use with Qubit.
    ///
    /// The `Marker` generic is a utility in order to provide implementations for `Fn` traits which
    /// take generics as parameters.
    pub trait QubitHandler<MParams, MReturn>: 'static + Send + Sync + Clone {
        /// Context type this handler expects.
        type Ctx: 'static + Send + Sync;
        /// Parameters that the handler will accept (excluding [`Ctx`](QubitHandler::Ctx)).
        type Params: TsTypeTuple;
        /// Return type of the handler.
        type Return;

        /// Call the handler with the provided `Ctx` and [`Params`]. The handler implementation
        /// must deserialise the parameters as required.
        fn call(&self, ctx: &Self::Ctx, params: Params) -> Self::Return;
    }

    macro_rules! impl_handlers {
        (impl [$($ctx:ident, $($params:ident,)*)?]) => {
            impl<F, R, $($ctx, $($params),*)?> QubitHandler<
                ($($ctx, $($params,)*)?), // MParams
                R
            >
            for F
            where
                F: 'static + Send + Sync + Clone + Fn($(&$ctx, $($params),*)?) -> R,
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
                    #[allow(unused)] params: Params
                ) -> Self::Return {
                    // If parameters are included, deserialise them.
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

                    // Call the handler, optionally with the context and any parameters.
                    self($(ctx, $($params,)*)?)
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

    pub mod meta {
        //! Utilities for passing handlers and associated information at run time.

        /// Kind of the handler. This will correspond with the method the user must call from
        /// TypeScript.
        #[derive(Clone, Debug)]
        pub enum HandlerKind {
            Query,
            Mutation,
            Subscription,
        }

        /// Static metadata associated with handler.
        ///
        ///  This should be generated with the [`handler`](crate::handler) macro.
        #[derive(Clone, Debug)]
        pub struct HandlerMeta {
            /// Kind of the handler.
            pub kind: HandlerKind,
            /// RPC name of the handler (this may differ from the name of the handler function).
            pub name: &'static str,
            /// Name of the parameters for this handler.
            pub param_names: &'static [&'static str],
        }

        /// All components of a handler required to initialise the
        /// [`RpcModule`](jsonrpsee::RpcModule), and generate TypeScript bindings for this handler.
        /// Instances of this struct can be called directly in order to invoke the underlying
        /// handler.
        ///
        /// This should be generated with the [`handler`](crate::handler) macro.
        #[derive(Clone)]
        pub struct HandlerDef<F> {
            /// Handler implementation.
            pub handler: F,
            /// Metadata for the handler.
            pub meta: HandlerMeta,
        }

        impl<F> std::ops::Deref for HandlerDef<F> {
            type Target = F;

            fn deref(&self) -> &Self::Target {
                &self.handler
            }
        }
    }

    mod async_shit {
        use std::{convert::Infallible, marker::PhantomData, pin::pin};

        use futures::{Stream, StreamExt};
        use jsonrpsee::RpcModule;
        use ts_rs::TS;

        mod response_value {
            use serde::Serialize;

            use super::*;

            /// Any Rust value that can be returned from a handler. It may require a transform function
            /// to turn it into a serialisable value.
            pub trait ResponseValue<MValue> {
                /// Serialisable value that will be produced.
                type Value: 'static + TS + Clone + Serialize;

                /// Transform into a serialisable value.
                fn transform(self) -> Self::Value;

                fn debug() -> String;
            }

            /// Marker for anything that implements [`TS`].
            ///
            /// As a [`ResponseValue`], these values can be directly returned without any
            /// transformation.
            pub struct MTs;
            impl<T> ResponseValue<MTs> for T
            where
                T: 'static + TS + Clone + Serialize,
            {
                type Value = Self;

                fn transform(self) -> Self::Value {
                    self
                }

                fn debug() -> String {
                    "TS".to_string()
                }
            }

            /// Marker for anything that implements [`Iterator`].
            ///
            /// As a [`ResponseValue`], the iterator will be collected into a `Vec` before being
            /// returned.
            ///
            /// The `MValue` generic is a marker for the value contained within the iterator.
            pub struct MIter<MValue>(PhantomData<MValue>);
            impl<T, MValue> ResponseValue<MIter<MValue>> for T
            where
                T: Iterator,
                T::Item: ResponseValue<MValue>,
            {
                type Value = Vec<<T::Item as ResponseValue<MValue>>::Value>;

                fn transform(self) -> Self::Value {
                    self.map(|value| value.transform()).collect()
                }

                fn debug() -> String {
                    format!("Iter<{}>", T::Item::debug())
                }
            }
        }
        pub use response_value::*;

        use super::QubitHandler;

        /// Registration implementation differs depending on the return type of the handler. This
        /// is to account for handlers which may return futures, streams, or values directly.
        pub trait RegisterableHandler<MParams, MReturn, MValue, M>:
            QubitHandler<MParams, MReturn>
        {
            type Output: ResponseValue<MValue>;

            fn register(self, module: &mut RpcModule<Self::Ctx>, method_name: String);
        }

        pub struct ActualValue<MValue>(PhantomData<MValue>);
        impl<T, MValue, MParams, MReturn>
            RegisterableHandler<MParams, MReturn, MValue, ActualValue<MValue>> for T
        where
            T: QubitHandler<MParams, MReturn>,
            T::Return: ResponseValue<MValue>,
        {
            type Output = T::Return;

            fn register(self, module: &mut RpcModule<Self::Ctx>, method_name: String) {
                println!("registering actual value");
                module
                    .register_blocking_method(
                        Box::leak(method_name.into_boxed_str()),
                        move |params, ctx, _extensions| {
                            let result = self.call(&ctx, params);
                            Ok::<_, Infallible>(result.transform())
                        },
                    )
                    .unwrap();
            }
        }

        pub struct MFuture<MOut>(PhantomData<MOut>);
        impl<T, MValue, MParams, MReturn>
            RegisterableHandler<MParams, MReturn, MValue, MFuture<ActualValue<MValue>>> for T
        where
            T: QubitHandler<MParams, MReturn>,
            T::Return: Future + Send,
            <T::Return as Future>::Output: ResponseValue<MValue>,
        {
            type Output = <T::Return as Future>::Output;

            fn register(self, module: &mut RpcModule<Self::Ctx>, method_name: String) {
                println!("registering future");
                module
                    .register_async_method(
                        Box::leak(method_name.into_boxed_str()),
                        move |params, ctx, _extensions| {
                            let f = self.clone();

                            async move {
                                let result = f.call(&ctx, params).await;
                                Ok::<_, Infallible>(result.transform())
                            }
                        },
                    )
                    .unwrap();
            }
        }

        pub struct MStream<MItem>(PhantomData<MItem>);
        impl<T, MValue, MParams, MReturn>
            RegisterableHandler<MParams, MReturn, MValue, MStream<ActualValue<MValue>>> for T
        where
            T: QubitHandler<MParams, MReturn>,
            T::Return: Stream + Send,
            <T::Return as Stream>::Item: Send + ResponseValue<MValue>,
        {
            type Output = <T::Return as Stream>::Item;

            fn register(self, module: &mut RpcModule<Self::Ctx>, method_name: String) {
                let notif_method_name = format!("{method_name}_notif");
                let unsub_method_name = format!("{method_name}_unsub");

                module
                    .register_subscription(
                        Box::leak(method_name.into_boxed_str()),
                        Box::leak(notif_method_name.into_boxed_str()),
                        Box::leak(unsub_method_name.into_boxed_str()),
                        move |params, pending, ctx, _extensions| {
                            let f = self.clone();

                            async move {
                                let sink = pending.accept().await.unwrap();

                                let mut stream = pin!(f.call(&ctx, params));

                                while let Some(item) = stream.next().await {
                                    let item =
                                        serde_json::value::to_raw_value(&item.transform()).unwrap();
                                    sink.send(item).await.unwrap();
                                }

                                Ok(())
                            }
                        },
                    )
                    .unwrap();
            }
        }

        impl<T, MValue, MParams, MReturn>
            RegisterableHandler<MParams, MReturn, MValue, MFuture<MStream<ActualValue<MValue>>>>
            for T
        where
            T: QubitHandler<MParams, MReturn>,
            T::Return: Send + Future,
            <T::Return as Future>::Output: Stream + Send,
            <<T::Return as Future>::Output as Stream>::Item: Send + ResponseValue<MValue>,
        {
            type Output = <<T::Return as Future>::Output as Stream>::Item;

            fn register(self, module: &mut RpcModule<Self::Ctx>, method_name: String) {
                let notif_method_name = format!("{method_name}_notif");
                let unsub_method_name = format!("{method_name}_unsub");

                module
                    .register_subscription(
                        Box::leak(method_name.into_boxed_str()),
                        Box::leak(notif_method_name.into_boxed_str()),
                        Box::leak(unsub_method_name.into_boxed_str()),
                        move |params, pending, ctx, _extensions| {
                            let f = self.clone();

                            async move {
                                let sink = pending.accept().await.unwrap();

                                let mut stream = pin!(f.call(&ctx, params).await);

                                while let Some(item) = stream.next().await {
                                    let item =
                                        serde_json::value::to_raw_value(&item.transform()).unwrap();
                                    sink.send(item).await.unwrap();
                                }

                                Ok(())
                            }
                        },
                    )
                    .unwrap();
            }
        }

        fn register_fn_3<Ctx, MParams, MReturn, MWhatever, MValue>(
            module: &mut RpcModule<Ctx>,
            method_name: String,
            handler: impl RegisterableHandler<MParams, MReturn, MValue, MWhatever, Ctx = Ctx>,
        ) where
            Ctx: 'static + Send + Sync,
        {
            handler.register(module, method_name);
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
                fn register_handler<F, MParams, MReturn, MValue, M>(handler: F) -> RpcModule<()>
                where
                    F: RegisterableHandler<MParams, MReturn, MValue, M, Ctx = ()>,
                {
                    let mut module = RpcModule::new(());
                    F::register(handler, &mut module, "handler".to_string());
                    module
                }

                /// Register a handler to a module, and call it, returning the value that was
                /// returned from the handler according to [`ReturnType`].
                async fn test_handler<F, MParams, MReturn, MValue, M>(
                    handler: F,
                ) -> <F::Output as ResponseValue<MValue>>::Value
                where
                    F: RegisterableHandler<MParams, MReturn, MValue, M, Ctx = ()>,
                    <F::Output as ResponseValue<MValue>>::Value: for<'a> Deserialize<'a>,
                {
                    let module = register_handler(handler);

                    let fut = module.call::<[(); 0], <F::Output as ResponseValue<MValue>>::Value>(
                        "handler",
                        [],
                    );
                    fut.await.unwrap()
                }

                /// Primitive `TS` values should be returned as-is.
                #[tokio::test]
                async fn ts() {
                    assert_eq!(test_handler(|| 123u32).await, 123);
                }

                /// Iterators should be collected and returned as a `Vec`.
                #[tokio::test]
                async fn iter() {
                    assert_eq!(test_handler(simple_iter).await, vec![0, 1, 2]);
                }

                /// Stream should be consumed and each value returned one at a time.
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

            #[test]
            fn do_something() {
                use futures::stream;
                use std::iter;

                fn register_fn<MParams, MReturn, MWhatever, MValue>(
                    handler: impl RegisterableHandler<MParams, MReturn, MValue, MWhatever, Ctx = ()>,
                ) {
                    register_fn_3(&mut RpcModule::new(()), "handler".to_string(), handler);
                }

                register_fn(|| 123);
                register_fn(|| async { 123 });
                register_fn(|| stream::once(async { 123 }));
                register_fn(|| async { stream::once(async { 123 }) });
                register_fn(|| iter::once(123));
                register_fn(|| async { iter::once(123) });
                register_fn(|| stream::once(async { iter::once(123) }));
                register_fn(|| async { stream::once(async { iter::once(123) }) });
                register_fn(|| iter::once(iter::once(123)));
                register_fn(|| async { iter::once(iter::once(123)) });
                register_fn(|| stream::once(async { iter::once(iter::once(123)) }));
                register_fn(|| async { stream::once(async { iter::once(iter::once(123)) }) });
            }
        }
    }

    pub use async_shit::*;

    #[cfg(test)]
    mod test {
        use serde_json::{Value, json};

        use super::*;

        /// Call a [`QubitHandler`], and return its return value.
        fn call_handler<H, MParams, MReturn>(handler: H, params: Value) -> H::Return
        where
            H: QubitHandler<MParams, MReturn, Ctx = ()>,
        {
            QubitHandler::call(
                &handler,
                &(),
                Params::new(Some(&serde_json::to_string(&params).unwrap())).into_owned(),
            )
        }

        #[test]
        fn call_paramless_handler() {
            fn handler() {}
            call_handler(handler, json!(()));
        }

        #[test]
        fn call_handler_with_ctx() {
            fn handler(_ctx: &()) {}
            call_handler(handler, json!(()));
        }

        #[test]
        fn call_handler_with_ctx_and_param() {
            fn handler(_ctx: &(), param: u32) -> u32 {
                param
            }
            assert_eq!(call_handler(handler, json!([123])), 123);
        }

        #[test]
        fn call_handler_with_ctx_and_params() {
            fn handler(_ctx: &(), param_1: u32, param_2: String) -> (u32, String) {
                (param_1, param_2)
            }
            assert_eq!(
                call_handler(handler, json!([123, "hello"])),
                (123, "hello".to_string())
            );
        }

        mod test_impl {
            //! Some random trait assertions for [`QubitHandler`].

            use super::*;

            use static_assertions::assert_impl_all;

            // Handler with no inputs/outputs.
            assert_impl_all!(
                fn () -> (): QubitHandler<(), (), Ctx = (), Params = (), Return = ()>
            );
            // Handler with single Ctx param.
            assert_impl_all!(
                fn (&u32) -> (): QubitHandler<(u32,), (), Ctx = u32, Params = (), Return = ()>
            );
            // Handler with Ctx param, and other parameters.
            assert_impl_all!(
                fn (&u32, String, bool) -> (): QubitHandler<(u32, String, bool), (), Ctx = u32, Params = (String, bool), Return = ()>
            );
            // Handler with primitive return type.
            assert_impl_all!(
                fn () -> u32: QubitHandler<(), u32, Ctx = (), Params = (), Return = u32>
            );
            // Handler with iterator return type.
            assert_impl_all!(
                fn () -> std::vec::IntoIter<u32> : QubitHandler<(), std::vec::IntoIter<u32>>
            );
            // Handler with stream return type.
            assert_impl_all!(
                fn () -> futures::stream::Iter<std::vec::IntoIter<u32>> : QubitHandler<(), futures::stream::Iter<std::vec::IntoIter<u32>>>
            );
            // Handler returning a stream of iterators of iterators.
            assert_impl_all!(
                fn () -> futures::stream::Iter<std::vec::IntoIter<std::vec::IntoIter<u32>>> : QubitHandler<(), futures::stream::Iter<std::vec::IntoIter<std::vec::IntoIter<u32>>>>
            );
        }
    }
}

use handler::*;

mod router {
    use jsonrpsee::RpcModule;

    use super::{handler::meta::*, *};

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
        pub fn handler<F, MParams, MReturn, MValue, M>(mut self, handler: HandlerDef<F>) -> Self
        where
            F: RegisterableHandler<MParams, MReturn, MValue, M, Ctx = Ctx>,
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
        ts_type::{TsType, TsTypeTuple},
        *,
    };

    fn assert_handler<F, HandlerMarker, ReturnMarker, MValue, M>(
        _handler: F,
        _expected_ctx: F::Ctx,
    ) -> (Vec<TsType>, TsType)
    where
        F: RegisterableHandler<HandlerMarker, ReturnMarker, MValue, M>,
    {
        (
            F::Params::get_ts_types(),
            TsType::from_type::<<F::Output as ResponseValue<_>>::Value>(),
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
