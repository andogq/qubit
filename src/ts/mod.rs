use std::{future::Future, marker::Send};

use jsonrpsee::types::Params;
use serde::Deserialize;
use ts_rs::TS;

mod ts_type {
    //! Utilities for representing TypeScript types at runtime.

    use std::{any::TypeId, convert::Infallible, ops::Deref, pin::pin};

    use derive_more::Deref;
    use futures::{Stream, StreamExt};
    use jsonrpsee::RpcModule;
    use serde::Serialize;
    use ts_rs::TS;

    use super::QubitHandler;

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

        /// Produce type information for the given handler return type.
        ///
        /// This is a utility to help avoid specifying the `Marker` generic present on
        /// [`HandlerReturnValue`].
        pub fn from_handler_return_type<T: HandlerReturnType<Marker>, Marker>() -> Self {
            T::inner_ts_type()
        }

        /// Produce type information for the given handler return value.
        ///
        /// This is a utility for when the type cannot be specified and
        /// [`Self::from_handler_return_type`] cannot be used.
        pub fn from_handler_return_value<T: HandlerReturnType<Marker>, Marker>(_value: T) -> Self {
            Self::from_handler_return_type::<T, _>()
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

    /// Represents any type of value that may be returned from a handler.
    ///
    /// The `Marker` allows this trait to be implemented on multiple traits. If there is a conflict
    /// in implementations, an error will be produced at the call site, rather than when
    /// implementing the trait.
    pub trait HandlerReturnType<Marker> {
        type Inner: 'static + Clone + TS + Serialize + Send;

        fn value(self) -> Self::Inner;

        /// Produce the [`TsType`] of the 'inner' value of the return type. This is the type that
        /// should be exposed to the user.
        fn inner_ts_type() -> TsType;

        fn register<Ctx, const PARAM_COUNT: usize, M2, RMarker>(
            module: &mut RpcModule<Ctx>,
            handler: impl QubitHandler<PARAM_COUNT, M2, RMarker, Ctx = Ctx, R = Self>,
            method_name: String,
        ) where
            Ctx: 'static + Send + Sync;
    }

    /// Marker for any type that implements [`TS`]. This will directly produce the [`TsType`]
    /// as-is.
    #[doc(hidden)]
    pub struct TsMarker;
    impl<T> HandlerReturnType<TsMarker> for T
    where
        T: 'static + Clone + TS + Serialize + Send,
    {
        type Inner = T;

        fn value(self) -> Self::Inner {
            self
        }

        fn inner_ts_type() -> TsType {
            TsType::from_type::<T>()
        }

        fn register<Ctx, const PARAM_COUNT: usize, M2, RMarker>(
            module: &mut RpcModule<Ctx>,
            handler: impl QubitHandler<PARAM_COUNT, M2, RMarker, Ctx = Ctx, R = Self>,
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

    /// Marker for any type that is an iterator of [`TS`] items. This will correspond with
    /// `Array<T>` in the generated TypeScript type.
    #[doc(hidden)]
    pub struct IterMarker;
    impl<T> HandlerReturnType<IterMarker> for T
    where
        T: Iterator,
        T::Item: 'static + Clone + TS + Serialize + Send,
    {
        type Inner = Vec<T::Item>;

        fn value(self) -> Self::Inner {
            self.collect()
        }

        fn inner_ts_type() -> TsType {
            TsType::from_type::<Self::Inner>()
        }

        fn register<Ctx, const PARAM_COUNT: usize, M2, RMarker>(
            module: &mut RpcModule<Ctx>,
            handler: impl QubitHandler<PARAM_COUNT, M2, RMarker, Ctx = Ctx, R = Self>,
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
    impl<T> HandlerReturnType<StreamMarker> for T
    where
        T: Stream + Send,
        T::Item: 'static + Clone + TS + Serialize + Send,
    {
        type Inner = T::Item;

        fn value(self) -> Self::Inner {
            // TODO: This should be streamed back, not collected.
            todo!()
        }

        fn inner_ts_type() -> TsType {
            // TODO: Should this return `Subscription<...>` at this point?
            TsType::from_type::<T::Item>()
        }

        fn register<Ctx, const PARAM_COUNT: usize, M2, RMarker>(
            module: &mut RpcModule<Ctx>,
            handler: impl QubitHandler<PARAM_COUNT, M2, RMarker, Ctx = Ctx, R = Self>,
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
        }

        mod handler_return_type {
            use super::*;

            #[test]
            fn ts() {
                let ts_type = TsType::from_handler_return_type::<u32, _>();
                assert_eq!(ts_type.name, "number");
                assert!(ts_type.is_primitive());
            }

            #[test]
            fn iter() {
                let ts_type = TsType::from_handler_return_value(std::iter::once(true));
                assert_eq!(ts_type.name, "Array<boolean>");
                assert!(ts_type.is_primitive());
            }

            #[test]
            fn stream() {
                let ts_type =
                    TsType::from_handler_return_value(futures::stream::once(async { "hello" }));
                assert_eq!(ts_type.name, "string");
                assert!(ts_type.is_primitive());
            }
        }
    }
}

use ts_type::*;

/// A handler suitable for use with Qubit.
///
/// The `Marker` generic is a utility in order to provide implementations for `Fn` traits which
/// take generics as parameters.
trait QubitHandler<const PARAM_COUNT: usize, Marker, RMarker>: 'static + Send + Sync + Clone {
    /// Context type this handler expects.
    type Ctx;

    type R: HandlerReturnType<RMarker>;

    /// Type information for parameters. This excludes the context parameter.
    fn param_tys() -> [TsType; PARAM_COUNT];

    /// Type information for the return type.
    fn return_ty() -> TsType;

    fn call(
        &self,
        ctx: &Self::Ctx,
        params: Params<'static>,
    ) -> impl Future<Output = Self::R> + Send + Sync;
}

macro_rules! impl_handlers {
    (impl [$($ctx:ident, $($params:ident,)*)?]) => {
        impl<F, R, RMarker, $($ctx, $($params),*)?> QubitHandler<
            { impl_handlers!(count [$($($params,)*)?]) },
            ($($ctx, $($params,)*)?),
            RMarker
        >
        for F
        where
            F: 'static + Send + Sync + Clone + Fn($(&$ctx, $($params),*)?) -> R,
            R: 'static + HandlerReturnType<RMarker>,
            $(
                $ctx: 'static + Send + Sync,
                $($params: 'static + TS + Send + for<'a> Deserialize<'a>),*
            )?
        {
            type Ctx = impl_handlers!(ctx_ty [$($ctx)?]);

            type R = R;

            fn param_tys() -> [TsType; { impl_handlers!(count [$($($params,)*)?]) }] {
                [
                    $($(TsType::from_type::<$params>(),)*)?
                ]
            }

            fn return_ty() -> TsType {
                TsType::from_handler_return_type::<R, _>()
            }

            fn call(&self, #[allow(unused)] ctx: &Self::Ctx, #[allow(unused)] params: Params<'static>) -> impl Future<Output = R> + Send + Sync {
                async move {
                    $(
                        #[allow(non_snake_case)]
                        let ($($params,)*) = match params.parse::<($($params,)*)>() {
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

    (eat []) => {};

    (eat [$param:ident, $($params:ident,)*]) => {
        impl_handlers!($($params),*);
    };

    (count []) => { 0 };

    (count [$param:ident, $($params:ident,)*]) => {
        1 + impl_handlers!(count [$($params,)*])
    };

    ($($params:ident),* $(,)?) => {
        impl_handlers!(impl [$($params,)*]);
        impl_handlers!(eat [$($params,)*]);
    };
}

impl_handlers!(
    P0, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11, P12, P13, P14, P15
);

#[derive(Clone, Debug)]
enum HandlerKind {
    Query,
    Mutation,
    Subscription,
}

#[derive(Clone, Debug)]
struct HandlerMeta {
    kind: HandlerKind,
    name: &'static str,
    param_names: &'static [&'static str],
}

#[derive(Clone)]
struct HandlerDef<F> {
    handler: F,
    meta: HandlerMeta,
}

impl<F> std::ops::Deref for HandlerDef<F> {
    type Target = F;

    fn deref(&self) -> &Self::Target {
        &self.handler
    }
}

// mod router {
//     use std::sync::Arc;

//     use futures::FutureExt;
//     use jsonrpsee::RpcModule;

//     use super::*;

//     type ErasedHandlerFn<Ctx> = Box<
//         dyn Fn(
//             Params<'static>,
//             Arc<Ctx>,
//             Extensions,
//         ) -> Pin<
//             Box<dyn Future<Output = ResponsePayload<'static, Value>> + Send + Sync + 'static>,
//         >,
//     >;
//     struct HandlerErased<Ctx> {
//         handler: ErasedHandlerFn<Ctx>,
//         meta: HandlerMeta,
//     }

//     struct Router<Ctx> {
//         nested_routers: Vec<(&'static str, Router<Ctx>)>,
//         handlers: Vec<HandlerErased<Ctx>>,
//     }

//     impl<Ctx> Router<Ctx>
//     where
//         Ctx: Clone + Send + Sync + 'static,
//     {
//         pub fn new() -> Self {
//             Self {
//                 nested_routers: Vec::new(),
//                 handlers: Vec::new(),
//             }
//         }

//         pub fn handler<const PARAM_COUNT: usize, F, HandlerMarker>(
//             mut self,
//             handler: HandlerDef<F>,
//         ) -> Self
//         where
//             F: QubitHandler<PARAM_COUNT, HandlerMarker, Ctx = Ctx> + Clone,
//         {
//             let HandlerDef { handler, meta } = handler;
//             let handler = Arc::new(handler);

//             self.handlers.push(HandlerErased {
//                 handler: Box::new(move |params, ctx, extensions| {
//                     let handler = handler.clone();

//                     async move { handler.call(*ctx, params).await.into_response() }.boxed()
//                 }),
//                 meta,
//             });

//             self
//         }

//         // TODO: Allow `Ctx` to be `Arc` or not.
//         fn to_module(self, ctx: Ctx) -> RpcModule<Ctx> {
//             let mut module = RpcModule::new(ctx);

//             self.visit_handlers(|name, handler| {
//                 module.register_async_method(name.as_str(), handler);
//             });

//             module
//         }

//         fn visit_handlers(self, mut visitor: impl FnMut(String, ErasedHandlerFn<Ctx>)) {
//             for erased in self.handlers {
//                 visitor(erased.meta.name.to_string(), erased.handler);
//             }

//             for (prefix, router) in self.nested_routers {
//                 router.visit_handlers(|name, handler| {
//                     visitor(format!("{prefix}.{name}"), handler);
//                 });
//             }
//         }
//     }
// }

mod router {
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
        pub fn handler<F, const PARAM_COUNT: usize, M, RM>(mut self, handler: HandlerDef<F>) -> Self
        where
            F: QubitHandler<PARAM_COUNT, M, RM, Ctx = Ctx>,
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

                F::R::register(module, handler.handler, method_name);
            }));

            self.handler_meta.push(handler.meta);

            self
        }

        pub fn into_module(mut self, ctx: Ctx) -> RpcModule<Ctx> {
            let mut module = RpcModule::new(ctx);

            // Add all nested routers first.
            for (prefix, router) in std::mem::take(&mut self.nested_routers) {
                router.add_to_module(&mut module, Some(&prefix));
            }

            // Finally, consume this router and add it.
            self.add_to_module(&mut module, None);

            module
        }

        fn add_to_module(self, module: &mut RpcModule<Ctx>, prefix: Option<&str>) {
            for register in self.register_methods {
                register(module, prefix);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use futures::{Stream, stream};

    use super::*;

    fn assert_handler<const PARAM_COUNT: usize, F, HandlerMarker, ReturnMarker>(
        _handler: F,
        _expected_ctx: F::Ctx,
    ) -> ([TsType; PARAM_COUNT], TsType)
    where
        F: QubitHandler<PARAM_COUNT, HandlerMarker, ReturnMarker>,
    {
        (F::param_tys(), F::return_ty())
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
