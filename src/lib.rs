use std::collections::HashSet;

use futures::{Future, FutureExt, Stream, StreamExt};
use jsonrpsee::{server::StopHandle, types::Params, RpcModule, SubscriptionMessage};
pub use rs_ts_api_macros::*;
use server::ServerService;
use ts_rs::Dependency;

pub mod server;

pub struct ServerType {
    ty: String,
    dependencies: Vec<Dependency>,
}

impl ToString for ServerType {
    fn to_string(&self) -> String {
        let imports = self
            .dependencies
            .iter()
            .map(|dep| {
                format!(
                    "import type  {{ {} }} from \"./{}\";",
                    dep.ts_name,
                    dep.exported_to.trim_end_matches(".ts")
                )
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join("\n");

        format!("{imports}\n{}", self.ty)
    }
}

pub struct HandlerType {
    pub name: String,
    pub signature: String,
    pub dependencies: Vec<Dependency>,
}

pub trait Handler {
    fn register(rpc_builder: RpcBuilder) -> RpcBuilder;

    fn get_type() -> HandlerType;
}

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

pub struct Router {
    handler_types: Vec<fn() -> HandlerType>,
    handler_builders: Vec<fn(RpcBuilder) -> RpcBuilder>,
    nested_routers: Vec<(&'static str, Router)>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            handler_types: Vec::new(),
            handler_builders: Vec::new(),
            nested_routers: Vec::new(),
        }
    }

    pub fn handler<H: Handler>(mut self, _: H) -> Self {
        self.handler_builders.push(H::register);
        self.handler_types.push(H::get_type);

        self
    }

    pub fn nest(mut self, namespace: &'static str, router: Router) -> Self {
        self.nested_routers.push((namespace, router));

        self
    }

    pub fn get_type(&self) -> ServerType {
        let (mut handlers, mut dependencies) = self
            .handler_types
            .iter()
            .map(|get_type| get_type())
            .map(|handler_type| {
                (
                    format!("{}: {}", handler_type.name, handler_type.signature),
                    handler_type.dependencies,
                )
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();

        self.nested_routers
            .iter()
            .map(|(namespace, router)| (namespace, router.get_type()))
            .for_each(
                |(
                    namespace,
                    ServerType {
                        ty: router_type,
                        dependencies: router_deps,
                    },
                )| {
                    handlers.push(format!("{namespace}: {router_type}"));
                    dependencies.push(router_deps);
                },
            );

        // Generate the router type
        let router_type = format!("{{ {} }}", handlers.join(", "));

        // Merge all dependencies
        let dependencies = dependencies.into_iter().flatten().collect::<Vec<_>>();

        ServerType {
            ty: router_type,
            dependencies,
        }
    }

    pub fn build_rpc_module(self, namespace: Option<&'static str>) -> RpcModule<()> {
        let mut rpc_module = self
            .handler_builders
            .into_iter()
            .fold(
                RpcBuilder::with_namespace(namespace),
                |rpc_builder, builder| builder(rpc_builder),
            )
            .consume();

        self.nested_routers
            .into_iter()
            .map(|(namespace, router)| router.build_rpc_module(Some(namespace)))
            .for_each(|router| {
                rpc_module.merge(router).unwrap();
            });

        rpc_module
    }

    pub fn create_service(self, stop_handle: StopHandle) -> ServerService {
        let svc_builder = jsonrpsee::server::Server::builder().to_service_builder();

        // Create a top level module
        let rpc_module = self.build_rpc_module(None);

        ServerService {
            service: svc_builder.build(rpc_module, stop_handle),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[allow(non_camel_case_types)]
    struct sample_handler;
    impl Handler for sample_handler {
        fn register(_rpc_builder: RpcBuilder) -> RpcBuilder {
            todo!()
        }

        fn get_type() -> HandlerType {
            HandlerType {
                name: "sample_handler".to_string(),
                signature: "() => void".to_string(),
                dependencies: Vec::new(),
            }
        }
    }

    #[allow(non_camel_case_types)]
    struct another_handler;
    impl Handler for another_handler {
        fn register(_rpc_builder: RpcBuilder) -> RpcBuilder {
            todo!()
        }

        fn get_type() -> HandlerType {
            HandlerType {
                name: "another_handler".to_string(),
                signature: "() => number".to_string(),
                dependencies: Vec::new(),
            }
        }
    }

    #[test]
    fn empty_router() {
        let router = Router::new();
        let ty = router.get_type();

        assert_eq!(ty.ty, "{  }");
        assert_eq!(ty.dependencies, vec![]);
    }

    #[test]
    fn single_handler() {
        let router = Router::new().handler(sample_handler);
        let ty = router.get_type();

        assert_eq!(ty.ty, "{ sample_handler: () => void }");
        assert_eq!(ty.dependencies, vec![]);
    }

    #[test]
    fn multiple_handlers() {
        let router = Router::new()
            .handler(sample_handler)
            .handler(another_handler);
        let ty = router.get_type();

        assert_eq!(
            ty.ty,
            "{ sample_handler: () => void, another_handler: () => number }"
        );
        assert_eq!(ty.dependencies, vec![]);
    }
}
