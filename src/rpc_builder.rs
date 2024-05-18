use std::ops::Deref;

use futures::{Future, FutureExt, Stream, StreamExt};
use jsonrpsee::{
    types::{Params, ResponsePayload},
    IntoResponse, RpcModule, SubscriptionCloseResponse, SubscriptionMessage,
};
use serde::Serialize;
use serde_json::json;

use crate::FromContext;

pub struct RpcBuilder<Ctx> {
    namespace: Option<&'static str>,
    module: RpcModule<Ctx>,
}

impl<Ctx> RpcBuilder<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    pub fn new(ctx: Ctx) -> Self {
        Self::with_namespace(ctx, None)
    }

    pub fn namespaced(ctx: Ctx, namespace: &'static str) -> Self {
        Self::with_namespace(ctx, Some(namespace))
    }

    pub fn with_namespace(ctx: Ctx, namespace: Option<&'static str>) -> Self {
        Self {
            namespace,
            module: RpcModule::new(ctx),
        }
    }

    pub fn consume(self) -> RpcModule<Ctx> {
        self.module
    }

    fn namespace_str(&self, s: &'static str) -> &'static str {
        if let Some(namespace) = self.namespace {
            Box::leak(format!("{namespace}.{s}").into_boxed_str())
        } else {
            s
        }
    }

    pub fn query<T, R, C, F, Fut>(mut self, name: &'static str, handler: F) -> Self
    where
        T: Serialize + Clone + 'static,
        R: IntoResponse<Output = T> + 'static,
        C: FromContext<Ctx>,
        F: Fn(C, Params<'static>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = R> + Send + 'static,
    {
        self.module
            .register_async_method(self.namespace_str(name), move |params, ctx| {
                let handler = handler.clone();

                async move {
                    // Build the context
                    let ctx = match C::from_app_ctx(ctx.deref().clone()) {
                        Ok(ctx) => ctx,
                        Err(e) => {
                            // Handle any error building the context by turning it into a response
                            // payload.
                            return ResponsePayload::Error(e.into());
                        }
                    };

                    // Run the actual handler
                    handler(ctx, params).await.into_response()
                }
            })
            .unwrap();

        self
    }

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
                move |params, subscription, ctx| {
                    let handler = handler.clone();

                    async move {
                        // Accept the subscription
                        let subscription = subscription.accept().await.unwrap();

                        // Set up a channel to avoid cloning the subscription
                        let (tx, mut rx) = tokio::sync::mpsc::channel(10);

                        let mut count = 0;
                        let subscription_id = subscription.subscription_id();

                        // Recieve values on a new thread, sending them onwards to the subscription
                        tokio::spawn(async move {
                            while let Some(value) = rx.recv().await {
                                subscription
                                    .send(SubscriptionMessage::from_json(&value).unwrap())
                                    .await
                                    .unwrap();
                            }
                        });

                        // Build the context
                        let ctx = match C::from_app_ctx(ctx.deref().clone()) {
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
                        handler(ctx, params)
                            .await
                            .for_each(|value| {
                                count += 1;
                                tx.send(value).map(|result| result.unwrap())
                            })
                            .await;

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
}
