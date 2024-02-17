use std::ops::Deref;

use futures::{Future, FutureExt, Stream, StreamExt};
use jsonrpsee::{types::Params, RpcModule, SubscriptionMessage};

use crate::Context;

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

    pub fn query<C, F, Fut>(mut self, name: &'static str, handler: F) -> Self
    where
        C: Context<Ctx>,
        F: Fn(C, Params<'static>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = serde_json::Value> + Send + 'static,
    {
        self.module
            .register_async_method(self.namespace_str(name), move |params, ctx| {
                let handler = handler.clone();

                async move { handler(C::from_app_ctx(ctx.deref().clone()).unwrap(), params).await }
            })
            .unwrap();

        self
    }

    pub fn subscription<F, S>(
        mut self,
        name: &'static str,
        notification_name: &'static str,
        unsubscribe_name: &'static str,
        handler: F,
    ) -> Self
    where
        F: Fn(Ctx, Params<'static>) -> S + Send + Sync + Clone + 'static,
        S: Stream<Item = serde_json::Value> + Send + 'static,
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

                        // Recieve values on a new thread, sending them onwards to the subscription
                        tokio::spawn(async move {
                            while let Some(value) = rx.recv().await {
                                subscription
                                    .send(SubscriptionMessage::from_json(&value).unwrap())
                                    .await
                                    .unwrap();
                            }
                        })
                        .await
                        .unwrap();

                        // Run the handler, capturing each of the values sand forwarding it onwards
                        // to the channel
                        handler(ctx.deref().clone(), params)
                            .for_each(|value| tx.send(value).map(|result| result.unwrap()))
                            .await;
                    }
                },
            )
            .unwrap();

        self
    }
}
