use futures::{Future, FutureExt, Stream, StreamExt};
use jsonrpsee::{types::Params, RpcModule, SubscriptionMessage};

pub struct RpcBuilder {
    namespace: Option<&'static str>,
    module: RpcModule<()>,
}

impl RpcBuilder {
    pub fn new() -> Self {
        Self::with_namespace(None)
    }

    pub fn namespaced(namespace: &'static str) -> Self {
        Self::with_namespace(Some(namespace))
    }

    pub fn with_namespace(namespace: Option<&'static str>) -> Self {
        Self {
            namespace,
            module: RpcModule::new(()),
        }
    }

    pub fn consume(self) -> RpcModule<()> {
        self.module
    }

    fn namespace_str(&self, s: &'static str) -> &'static str {
        if let Some(namespace) = self.namespace {
            Box::leak(format!("{namespace}.{s}").into_boxed_str())
        } else {
            s
        }
    }

    pub fn query<F, Fut>(mut self, name: &'static str, handler: F) -> Self
    where
        F: Fn(Params<'static>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = serde_json::Value> + Send + 'static,
    {
        self.module
            .register_async_method(self.namespace_str(name), move |params, _ctx| {
                let handler = handler.clone();

                async move { handler(params).await }
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
        F: Fn(Params<'static>) -> S + Send + Sync + Clone + 'static,
        S: Stream<Item = serde_json::Value> + Send + 'static,
    {
        self.module
            .register_subscription(
                self.namespace_str(name),
                self.namespace_str(notification_name),
                self.namespace_str(unsubscribe_name),
                move |params, subscription, _ctx| {
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
                        handler(params)
                            .for_each(|value| tx.send(value).map(|result| result.unwrap()))
                            .await;
                    }
                },
            )
            .unwrap();

        self
    }
}
