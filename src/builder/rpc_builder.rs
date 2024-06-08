use std::ops::Deref;

use futures::{Future, Stream, StreamExt};
use jsonrpsee::{
    types::{Params, ResponsePayload},
    RpcModule, SubscriptionCloseResponse, SubscriptionMessage,
};
use serde::Serialize;
use serde_json::json;

use crate::FromContext;

/// Builder to construct the RPC module. Handlers can be registered using the [`RpcBuilder::query`]
/// and [`RpcBuilder::subscription`] methods. It tracks an internally mutable [`RpcModule`] and
/// it's namespace, ensuring that handlers names are correctly created.
///
/// For the most part, this should not be used manually, but rather with the [`qubit_macros::handler`]
/// macro.
pub struct RpcBuilder<Ctx> {
    /// The namespace for this module, which will be prepended onto handler names (if present).
    namespace: Option<&'static str>,

    /// The actual [`RpcModule`] that is being constructed.
    module: RpcModule<Ctx>,
}

impl<Ctx> RpcBuilder<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    /// Create a builder with the provided namespace.
    pub(crate) fn with_namespace(ctx: Ctx, namespace: Option<&'static str>) -> Self {
        Self {
            namespace,
            module: RpcModule::new(ctx),
        }
    }

    /// Consume the builder to produce the internal [`RpcModule`], ready to be used.
    pub(crate) fn build(self) -> RpcModule<Ctx> {
        self.module
    }

    /// Register a new query handler with the provided name.
    ///
    /// The `handler` can take its own `Ctx`, so long as it implements [`FromContext`]. It must
    /// return a future which outputs a serializable value.
    pub fn query<T, C, F, Fut>(mut self, name: &'static str, handler: F) -> Self
    where
        T: Serialize + Clone + 'static,
        C: FromContext<Ctx>,
        F: Fn(C, Params<'static>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        self.module
            .register_async_method(self.namespace_str(name), move |params, ctx, _extensions| {
                // NOTE: Handler has to be cloned in since `register_async_method` takes `Fn`, not
                // `FnOnce`. Not sure if it's better to be an `Rc`/leaked/???
                let handler = handler.clone();

                async move {
                    // Build the context
                    let ctx = match C::from_app_ctx(ctx.deref().clone()).await {
                        Ok(ctx) => ctx,
                        Err(e) => {
                            // Handle any error building the context by turning it into a response
                            // payload.
                            return ResponsePayload::Error(e.into());
                        }
                    };

                    // Run the actual handler
                    ResponsePayload::success(handler(ctx, params).await)
                }
            })
            .unwrap();

        self
    }

    /// Register a new subscription handler with the provided name.
    ///
    /// The `handler` can take its own `Ctx`, so long as it implements [`FromContext`]. It must
    /// return a future that outputs a stream of serializable values.
    pub fn subscription<T, C, F, Fut, S>(
        mut self,
        name: &'static str,
        notification_name: &'static str,
        unsubscribe_name: &'static str,
        handler: F,
    ) -> Self
    where
        T: Serialize + Send + Clone + 'static,
        C: FromContext<Ctx>,
        F: Fn(C, Params<'static>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = S> + Send + 'static,
        S: Stream<Item = T> + Send + 'static,
    {
        self.module
            .register_subscription(
                self.namespace_str(name),
                self.namespace_str(notification_name),
                self.namespace_str(unsubscribe_name),
                move |params, subscription, ctx, _extensions| {
                    // NOTE: Same deal here with cloning the handler as in the query registration.
                    let handler = handler.clone();

                    async move {
                        // Accept the subscription
                        let subscription = subscription.accept().await.unwrap();

                        // Set up a channel to avoid cloning the subscription
                        let (tx, mut rx) = tokio::sync::mpsc::channel(10);

                        // Track the number of items emitted through the subscription
                        let mut count = 0;
                        let subscription_id = subscription.subscription_id();

                        // Recieve values on a new thread, sending them onwards to the subscription
                        tokio::spawn(async move {
                            while let Some(value) = rx.recv().await {
                                if subscription.is_closed() {
                                    // Don't continue processing items once the web socket is
                                    // closed
                                    break;
                                }

                                subscription
                                    .send(SubscriptionMessage::from_json(&value).unwrap())
                                    .await
                                    .unwrap();
                            }
                        });

                        // Build the context
                        // NOTE: It won't be held across await so that `C` doesn't have to be
                        // `Send`
                        let ctx = match C::from_app_ctx(ctx.deref().clone()).await {
                            Ok(ctx) => ctx,
                            Err(e) => {
                                // Handle any error building the context by turning it into a
                                // subscriptions close response
                                return SubscriptionCloseResponse::NotifErr(
                                    SubscriptionMessage::from_json(&e).unwrap(),
                                );
                            }
                        };

                        // Run the handler, capturing each of the values sand forwarding it onwards
                        // to the channel
                        let mut stream = Box::pin(handler(ctx, params).await);

                        while let Some(value) = stream.next().await {
                            if tx.send(value).await.is_ok() {
                                count += 1;
                            } else {
                                break;
                            }
                        }

                        // Notify that stream is closing
                        SubscriptionCloseResponse::Notif(
                            SubscriptionMessage::from_json(
                                &json!({ "close_stream": subscription_id, "count": count }),
                            )
                            .unwrap(),
                        )
                    }
                },
            )
            .unwrap();

        self
    }

    /// Helper to 'resolve' some string with the namespace of this module (if it's present)
    fn namespace_str(&self, s: &'static str) -> &'static str {
        if let Some(namespace) = self.namespace {
            Box::leak(format!("{namespace}.{s}").into_boxed_str())
        } else {
            s
        }
    }
}
