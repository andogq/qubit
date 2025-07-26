pub mod ctx;
pub mod marker;
pub mod reflection;
pub mod response;
pub mod ts;

use futures::{Stream, StreamExt};
use jsonrpsee::{
    RpcModule, SubscriptionCloseResponse, SubscriptionMessage,
    types::{Params, ResponsePayload},
};
use serde::Deserialize;
use serde_json::json;
use ts_rs::TS;

use std::pin::pin;

use self::{ctx::FromRequestExtensions, response::ResponseValue, ts::TsTypeTuple};

/// A handler suitable for use with Qubit.
///
/// The `Marker` generic is a utility in order to provide implementations for `Fn` traits which
/// take generics as parameters.
pub trait QubitHandler<Ctx, MSig>: 'static + Send + Sync + Clone {
    /// Context type this handler expects.
    type Ctx: 'static + Send + Sync + FromRequestExtensions<Ctx>;
    /// Parameters that the handler will accept (excluding [`Ctx`](QubitHandler::Ctx)).
    type Params: TsTypeTuple;
    /// Return type of the handler.
    type Return;

    /// Call the handler with the provided `Ctx` and [`Params`]. The handler implementation
    /// must deserialise the parameters as required.
    fn call(&self, ctx: Self::Ctx, params: Params) -> Self::Return;
}

macro_rules! impl_handlers {
    (impl [$ctx:ident, $($params:ident,)*]) => {
        impl<Ctx, F, R, $ctx, $($params),*> QubitHandler<
            Ctx,
            (
                ($ctx, $($params,)*),
                R
            )
        >
        for F
        where
            F: 'static + Send + Sync + Clone + Fn($ctx, $($params),*) -> R,
            $ctx: 'static + Send + Sync + FromRequestExtensions<Ctx>,
            $($params: 'static + TS + Send + for<'a> Deserialize<'a>),*
        {
            type Ctx = $ctx;

            type Params = ($($params,)*);
            type Return = R;

            fn call(
                &self,
                #[allow(unused)] ctx: Self::Ctx,
                #[allow(unused)] params: Params
            ) -> Self::Return {
                #[allow(non_snake_case)]
                let ($($params,)*) = match impl_handlers!(parse_impl params -> [$($params,)*]) {
                    Ok(params) => params,
                    Err(e) => {
                        // TODO: Something
                        dbg!(e);
                        panic!("fukc");
                    }
                };

                // Call the handler, optionally with the context and any parameters.
                self(ctx, $($params,)*)
            }
        }
    };

    // HACK: This is to work around `serde_json` not allowing parsing `()` from `[]`:
    //
    // ```rs
    // serde_json::from_str::<()>("[]")
    // ```
    //
    // Instead, this swaps between two implementations. If the handler takes no parameters, it will
    // try parse `[(); 0]` (to ensure that no unnecessary parameters were passed to the handler).
    // Otherwise if the parameter does require parameters, they will be parsed as normal.
    //
    // This can be reverted once the following PR lands: https://github.com/serde-rs/json/pull/869
    (parse_impl $params:ident -> []) => {
        $params.parse::<[(); 0]>()
            .map(|_| ())
    };
    (parse_impl $params:ident -> [$($param_tys:ident,)*]) => {
        $params.parse::<Self::Params>()
    };

    (count []) => { 0 };
    (count [$param:ident, $($params:ident,)*]) => {
        1 + impl_handlers!(count [$($params,)*])
    };

    (recurse [$param:ident,]) => {};
    (recurse [$param:ident, $($params:ident,)+]) => {
        impl_handlers!($($params),+);
    };

    ($($params:ident),+ $(,)?) => {
        impl_handlers!(impl [$($params,)+]);
        impl_handlers!(recurse [$($params,)+]);
    };
}

impl_handlers!(
    P0, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11, P12, P13, P14, P15
);

impl<F, R, Ctx> QubitHandler<Ctx, ((), R)> for F
where
    F: 'static + Send + Sync + Clone + Fn() -> R,
    Ctx: 'static + Send + Sync,
{
    type Ctx = Ctx;

    type Params = ();
    type Return = R;

    fn call(
        &self,
        #[allow(unused)] ctx: Self::Ctx,
        #[allow(unused)] params: Params,
    ) -> Self::Return {
        #[allow(non_snake_case)]
        match impl_handlers!(parse_impl params -> []) {
            Ok(params) => params,
            Err(e) => {
                // TODO: Something
                dbg!(e);
                panic!("fukc");
            }
        };

        // Call the handler, optionally with the context and any parameters.
        self()
    }
}

/// Registration implementation differs depending on the return type of the handler. This
/// is to account for handlers which may return futures, streams, or values directly.
pub trait RegisterableHandler<
    Ctx,
    MSig,
    MValue: marker::ResponseMarker,
    MReturn: marker::HandlerReturnMarker,
>: QubitHandler<Ctx, MSig>
{
    /// The 'response' of the handler, which might not necessarily be the direct return type of the
    /// handler. It may be the [`Future::Output`], a [`Stream::Item`], or some other value that is
    /// derived from a handler return value.
    type Response: ResponseValue<MValue>;

    /// Register this handler against the provided RPC module.
    fn register(self, module: &mut RpcModule<Ctx>, method_name: String);
}

/// Register any handler that directly returns a [`ResponseValue`]. This will generally be the
/// simplest of handlers, without any streaming or futures.
impl<Ctx, T, MSig, MValue> RegisterableHandler<Ctx, MSig, MValue, marker::MResponse<MValue>> for T
where
    Ctx: 'static + Clone + Send + Sync,
    MValue: marker::ResponseMarker,
    T: QubitHandler<Ctx, MSig>,
    T::Return: ResponseValue<MValue>,
{
    /// The response is whatever is returned from the handler (plus any additional processing from
    /// [`ResponseValue::transform`]).
    type Response = T::Return;

    /// These handlers will be registered using [`RpcModule::register_blocking_method`], so that
    /// the handler can be run on a new thread without blocking the server.
    fn register(self, module: &mut RpcModule<Ctx>, method_name: String) {
        module
            .register_async_method(
                Box::leak(method_name.into_boxed_str()),
                move |params, ctx, extensions| {
                    let handler = self.clone();

                    async move {
                        let ctx =
                            match Self::Ctx::from_request_extensions((*ctx).clone(), extensions)
                                .await
                            {
                                Ok(ctx) => ctx,
                                Err(e) => {
                                    return ResponsePayload::error(e);
                                }
                            };
                        let result = handler.call(ctx, params);
                        ResponsePayload::success(result.transform())
                    }
                },
            )
            .unwrap();
    }
}

/// Register any handler that returns a [`Future`] which outputs a [`ResponseValue`]. This
/// implementation covers `async` handlers.
impl<Ctx, T, MSig, MValue>
    RegisterableHandler<Ctx, MSig, MValue, marker::MFuture<marker::MResponse<MValue>>> for T
where
    Ctx: 'static + Clone + Send + Sync,
    MValue: marker::ResponseMarker,
    T: QubitHandler<Ctx, MSig>,
    T::Return: Future + Send,
    <T::Return as Future>::Output: ResponseValue<MValue>,
{
    /// The response will be the `await`ed value of the returned future.
    type Response = <T::Return as Future>::Output;

    /// These handlers will be registered using [`RpcModule::register_async_method`].
    fn register(self, module: &mut RpcModule<Ctx>, method_name: String) {
        module
            .register_async_method(
                Box::leak(method_name.into_boxed_str()),
                move |params, ctx, extensions| {
                    let f = self.clone();

                    async move {
                        let ctx =
                            match Self::Ctx::from_request_extensions((*ctx).clone(), extensions)
                                .await
                            {
                                Ok(ctx) => ctx,
                                Err(e) => {
                                    return ResponsePayload::error(e);
                                }
                            };
                        let result = f.call(ctx, params).await;
                        ResponsePayload::success(result.transform())
                    }
                },
            )
            .unwrap();
    }
}

/// Register any handler that returns a [`Stream`] containing items implementing [`ResponseValue`].
/// This implementation will only handle [`Stream`]s which are directly returned from a handler
/// (not async handlers).
impl<Ctx, T, MValue, MSig> RegisterableHandler<Ctx, MSig, MValue, marker::MStream<MValue>> for T
where
    Ctx: 'static + Clone + Send + Sync,
    MValue: marker::ResponseMarker,
    T: QubitHandler<Ctx, MSig>,
    T::Return: Stream + Send,
    <T::Return as Stream>::Item: Send + ResponseValue<MValue>,
{
    /// The response is the [`Stream::Item`] of the resulting stream. This response value will be
    /// produced multiple times.
    type Response = <T::Return as Stream>::Item;

    /// These handlers will be registered usig [`RpcModule::register_subscription`].
    fn register(self, module: &mut RpcModule<Ctx>, method_name: String) {
        let notif_method_name = format!("{method_name}_notif");
        let unsub_method_name = format!("{method_name}_unsub");

        module
            .register_subscription(
                Box::leak(method_name.into_boxed_str()),
                Box::leak(notif_method_name.into_boxed_str()),
                Box::leak(unsub_method_name.into_boxed_str()),
                move |params, pending, ctx, extensions| {
                    let f = self.clone();

                    async move {
                        let ctx =
                            match Self::Ctx::from_request_extensions((*ctx).clone(), extensions)
                                .await
                            {
                                Ok(ctx) => ctx,
                                Err(e) => {
                                    pending.reject(e).await;
                                    return SubscriptionCloseResponse::None;
                                }
                            };

                        let sink = pending.accept().await.unwrap();

                        // Track the number of items emitted through the subscription
                        let mut count = 0;
                        let subscription_id = sink.subscription_id();

                        let mut stream = pin!(f.call(ctx, params));

                        while let Some(item) = stream.next().await {
                            let item = serde_json::value::to_raw_value(&item.transform()).unwrap();
                            sink.send(item).await.unwrap();
                            count += 1;
                        }

                        // Notify that stream is closing
                        SubscriptionCloseResponse::Notif(SubscriptionMessage::from(
                            serde_json::value::to_raw_value(
                                &json!({ "close_stream": subscription_id, "count": count }),
                            )
                            .unwrap(),
                        ))
                    }
                },
            )
            .unwrap();
    }
}

// TODO: Combine the duplicated `register_subscription` logic between sync and async streams.

/// Register any handler that returns a [`Future`] that outputs a [`Stream`] containing items
/// implementing [`ResponseValue`]. This implementation only supports async handlers.
impl<Ctx, T, MValue, MSig>
    RegisterableHandler<Ctx, MSig, MValue, marker::MFuture<marker::MStream<MValue>>> for T
where
    Ctx: 'static + Clone + Send + Sync,
    MValue: marker::ResponseMarker,
    T: QubitHandler<Ctx, MSig>,
    T::Return: Send + Future,
    <T::Return as Future>::Output: Stream + Send,
    <<T::Return as Future>::Output as Stream>::Item: Send + ResponseValue<MValue>,
{
    type Response = <<T::Return as Future>::Output as Stream>::Item;

    fn register(self, module: &mut RpcModule<Ctx>, method_name: String) {
        let notif_method_name = format!("{method_name}_notif");
        let unsub_method_name = format!("{method_name}_unsub");

        module
            .register_subscription(
                Box::leak(method_name.into_boxed_str()),
                Box::leak(notif_method_name.into_boxed_str()),
                Box::leak(unsub_method_name.into_boxed_str()),
                move |params, pending, ctx, extensions| {
                    let f = self.clone();

                    async move {
                        let ctx =
                            match Self::Ctx::from_request_extensions((*ctx).clone(), extensions)
                                .await
                            {
                                Ok(ctx) => ctx,
                                Err(e) => {
                                    pending.reject(e).await;
                                    return SubscriptionCloseResponse::None;
                                }
                            };

                        let sink = pending.accept().await.unwrap();

                        // Track the number of items emitted through the subscription
                        let mut count = 0;
                        let subscription_id = sink.subscription_id();

                        let mut stream = pin!(f.call(ctx, params).await);

                        while let Some(item) = stream.next().await {
                            let item = serde_json::value::to_raw_value(&item.transform()).unwrap();
                            sink.send(item).await.unwrap();
                            count += 1;
                        }

                        // Notify that stream is closing
                        SubscriptionCloseResponse::Notif(SubscriptionMessage::from(
                            serde_json::value::to_raw_value(
                                &json!({ "close_stream": subscription_id, "count": count }),
                            )
                            .unwrap(),
                        ))
                    }
                },
            )
            .unwrap();
    }
}

#[cfg(test)]
mod test {
    use crate::RpcError;

    use super::{ctx::FromRequestExtensions, ts::TsType, *};

    use futures::stream;
    use rstest::rstest;
    use serde_json::{Value, json};

    use std::{fmt::Debug, iter};

    mod register {
        //! Test registering different kinds of handlers to a [`RpcModule`], and call them to
        //! ensure they produce the correct response.

        use jsonrpsee::RpcModule;
        use serde::Deserialize;

        use super::*;

        /// Produce an iterator counting from 0 to 2 (inclusive).
        fn simple_iter() -> impl Iterator<Item = usize> {
            0..3
        }

        /// Register a handler to a module, and return the module. The handler will be
        /// registered at `handler`.
        fn register_handler<
            F,
            MSig,
            MValue: marker::ResponseMarker,
            MReturn: marker::HandlerReturnMarker,
        >(
            handler: F,
        ) -> RpcModule<()>
        where
            F: RegisterableHandler<(), MSig, MValue, MReturn, Ctx = ()>,
        {
            let mut module = RpcModule::new(());
            F::register(handler, &mut module, "handler".to_string());
            module
        }

        /// Register a handler to a module, and call it, returning the value that was
        /// returned from the handler according to [`ReturnType`].
        async fn test_handler<
            F,
            MSig,
            MValue: marker::ResponseMarker,
            MReturn: marker::HandlerReturnMarker,
        >(
            handler: F,
        ) -> <F::Response as ResponseValue<MValue>>::Value
        where
            F: RegisterableHandler<(), MSig, MValue, MReturn, Ctx = ()>,
            <F::Response as ResponseValue<MValue>>::Value: for<'a> Deserialize<'a>,
        {
            let module = register_handler(handler);

            let fut = module
                .call::<[(); 0], <F::Response as ResponseValue<MValue>>::Value>("handler", []);
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

            // Stream should be over, so the summary object should be sent.
            assert_eq!(
                subs.next::<Value>().await.unwrap().unwrap().0["count"]
                    .as_i64()
                    .unwrap(),
                3
            );
            assert!(subs.next::<Value>().await.is_none());
        }
    }

    /// Register a bunch of different complex handler types.
    #[rstest]
    #[case::ts_value(|| 123)]
    #[case::async_ts_value(|| async { 123 })]
    #[case::stream(|| stream::once(async { 123 }))]
    #[case::async_stream(|| async { stream::once(async { 123 }) })]
    #[case::iter(|| iter::once(123))]
    #[case::async_iter(|| async { iter::once(123) })]
    #[case::stream_iter(|| stream::once(async { iter::once(123) }))]
    #[case::async_stream_iter(|| async { stream::once(async { iter::once(123) }) })]
    #[case::iter_iter(|| iter::once(iter::once(123)))]
    #[case::async_iter_iter(|| async { iter::once(iter::once(123)) })]
    #[case::stream_iter_iter(|| stream::once(async { iter::once(iter::once(123)) }))]
    #[case::async_stream_iter_iter(|| async { stream::once(async { iter::once(iter::once(123)) }) })]
    fn register_handler<
        MSig,
        MValue: marker::ResponseMarker,
        MReturn: marker::HandlerReturnMarker,
    >(
        #[case] handler: impl RegisterableHandler<(), MSig, MValue, MReturn, Ctx = ()>,
    ) {
        handler.register(&mut RpcModule::new(()), "handler".to_string());
    }

    /// Call some handlers, and assert the output.
    #[rstest]
    #[case(|| {}, json!([]), ())]
    #[case(|_ctx: ()| {}, json!([]), ())]
    #[case(|_ctx: (), param: u32| param, json!([123]), 123)]
    #[case(|_ctx: (), param_1: u32, param_2: String| -> (u32, String) { (param_1, param_2) }, json!([123, "hello"]), (123, "hello".to_string()))]
    fn call_handler<H, MSig>(#[case] handler: H, #[case] params: Value, #[case] expected: H::Return)
    where
        H: QubitHandler<(), MSig, Ctx = ()>,
        H::Return: Debug + PartialEq,
    {
        let output = handler.call(
            (),
            Params::new(Some(&serde_json::to_string(&params).unwrap())).into_owned(),
        );

        assert_eq!(output, expected);
    }

    /// Sample CTX.
    #[derive(Clone)]
    struct SampleCtx;

    /// Sample CTX that derives from [`SampleCtx`].
    #[derive(Clone)]
    struct DerivedCtx;
    impl FromRequestExtensions<SampleCtx> for DerivedCtx {
        async fn from_request_extensions(
            _ctx: SampleCtx,
            _extensions: http::Extensions,
        ) -> Result<Self, RpcError> {
            Ok(DerivedCtx)
        }
    }

    /// Ensure that a handler can be registered if the ctx can be derived from the module ctx.
    #[test]
    fn derived_ctx() {
        fn handler(_ctx: DerivedCtx) {}
        handler.register(&mut RpcModule::new(SampleCtx), "handler".to_string());
    }

    /// Assert that a handler implements [`RegisterableHandler`], and the reflected TS types are correct.
    #[rstest]
    #[case::unit_handler(|| {}, (), [], "null")]
    #[case::unit_handler_other_ctx(|| {}, SampleCtx, [], "null")]
    #[case::single_ctx_param(|_ctx: SampleCtx| {}, SampleCtx, [], "null")]
    #[case::only_return_ty(|| -> bool { todo!() }, (), [], "boolean")]
    #[case::ctx_and_param(|_ctx: SampleCtx, _a: u32| {}, SampleCtx, ["number"], "null")]
    #[case::ctx_and_param_and_return(|_ctx: SampleCtx, _a: u32| -> bool { todo!() }, SampleCtx, ["number"], "boolean")]
    #[case::ctx_and_multi_param(|_ctx: SampleCtx, _a: u32, _b: String, _c: bool| {}, SampleCtx, ["number", "string", "boolean"], "null")]
    #[case::ctx_and_multi_param_return(|_ctx: SampleCtx, _a: u32, _b: String, _c: bool| -> bool { todo!() }, SampleCtx, ["number", "string", "boolean"], "boolean")]
    #[case::produce_iter(|| { [1, 2, 3].into_iter() }, (), [], "Array<number>")]
    #[case::produce_stream(|| { stream::iter([1, 2, 3]) }, (), [], "number")]
    fn handler_ts_type<H, Ctx, MSig, MValue, MReturn>(
        #[case] _handler: H,
        #[case] _ctx: Ctx,
        #[case] expected_params: impl IntoIterator<Item = &'static str>,
        #[case] expected_return: &'static str,
    ) where
        MValue: marker::ResponseMarker,
        MReturn: marker::HandlerReturnMarker,
        H: RegisterableHandler<Ctx, MSig, MValue, MReturn>,
        Ctx: 'static + Clone + Send + Sync,
        H::Ctx: 'static + Send + Sync + FromRequestExtensions<Ctx>,
    {
        assert_eq!(
            H::Params::get_ts_types()
                .into_iter()
                .map(|ty| ty.name.clone())
                .collect::<Vec<_>>(),
            expected_params.into_iter().collect::<Vec<_>>()
        );
        assert_eq!(
            TsType::from_type::<<H::Response as ResponseValue<_>>::Value>().name,
            expected_return
        );
    }

    mod handler_traits {
        //! Some random trait assertions for [`QubitHandler`].

        use super::*;

        use static_assertions::assert_impl_all;

        // Handler with no inputs/outputs.
        assert_impl_all!(
            fn () -> (): QubitHandler<(), ((), ()), Ctx = (), Params = (), Return = ()>
        );
        // Handler with single Ctx param.
        assert_impl_all!(
            fn (u32) -> (): QubitHandler<u32, ((u32,), ()), Ctx = u32, Params = (), Return = ()>
        );
        // Handler with Ctx param, and other parameters.
        assert_impl_all!(
            fn (u32, String, bool) -> (): QubitHandler<u32, ((u32, String, bool), ()), Ctx = u32, Params = (String, bool), Return = ()>
        );
        // Handler with primitive return type.
        assert_impl_all!(
            fn () -> u32: QubitHandler<(), ((), u32), Ctx = (), Params = (), Return = u32>
        );
        // Handler with iterator return type.
        assert_impl_all!(
            fn () -> std::vec::IntoIter<u32> : QubitHandler<(), ((), std::vec::IntoIter<u32>)>
        );
        // Handler with stream return type.
        assert_impl_all!(
            fn () -> futures::stream::Iter<std::vec::IntoIter<u32>> : QubitHandler<(), ((), futures::stream::Iter<std::vec::IntoIter<u32>>)>
        );
        // Handler returning a stream of iterators of iterators.
        assert_impl_all!(
            fn () -> futures::stream::Iter<std::vec::IntoIter<std::vec::IntoIter<u32>>> : QubitHandler<(), ((), futures::stream::Iter<std::vec::IntoIter<std::vec::IntoIter<u32>>>)>
        );
    }
}
