use std::collections::HashSet;

use jsonrpsee::{server::StopHandle, RpcModule};
pub use rs_ts_api_macros::*;
use server::ServerService;
use ts_rs::Dependency;

pub mod server;

type RegisterHandlerFn = fn(RpcModule<()>) -> RpcModule<()>;

pub struct HandlerType {
    pub name: String,
    pub signature: String,
    pub dependencies: Vec<Dependency>,
}

pub trait Handler {
    fn register(router: jsonrpsee::RpcModule<()>) -> jsonrpsee::RpcModule<()>;

    fn get_type() -> HandlerType;
}

pub struct Router {
    name: Option<String>,
    handlers: Vec<(fn() -> HandlerType, RegisterHandlerFn)>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            name: None,
            handlers: Vec::new(),
        }
    }

    pub fn namespace(name: impl ToString) -> Self {
        Self {
            name: Some(name.to_string()),
            handlers: Vec::new(),
        }
    }

    pub fn handler<H: Handler>(mut self, _: H) -> Self {
        self.handlers.push((H::get_type, H::register));

        self
    }

    pub fn get_type(&self) -> String {
        let (handlers, dependencies) = self
            .handlers
            .iter()
            .map(|(get_type, _)| get_type())
            .map(|handler_type| {
                (
                    format!("{}: {}", handler_type.name, handler_type.signature),
                    handler_type.dependencies,
                )
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();

        // Generate the router type
        let mut router_type = format!("{{ {} }}", handlers.join(", "));

        // Merge all dependencies
        let dependencies = dependencies
            .into_iter()
            .flatten()
            .map(|dependency| {
                format!(
                    "import type {{ {} }} from \"./{}\";",
                    dependency.ts_name,
                    dependency.exported_to.trim_end_matches(".ts"),
                )
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        if let Some(name) = &self.name {
            router_type = format!("{{ {name}: {router_type} }}");
        }

        format!("{}\ntype Router = {router_type};", dependencies.join("\n"))
    }

    pub fn create_service(self, stop_handle: StopHandle) -> ServerService {
        let svc_builder = jsonrpsee::server::Server::builder().to_service_builder();

        let rpc_module = self
            .handlers
            .into_iter()
            .fold(RpcModule::new(()), |rpc_module, (_, register)| {
                register(rpc_module)
            });

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
        fn register(_router: jsonrpsee::RpcModule<()>) -> jsonrpsee::RpcModule<()> {
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
        fn register(_router: jsonrpsee::RpcModule<()>) -> jsonrpsee::RpcModule<()> {
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
        assert_eq!(router.get_type(), "{  }");
    }

    #[test]
    fn namespaced_empty_router() {
        let router = Router::namespace("ns");
        assert_eq!(router.get_type(), "{ ns: {  } }");
    }

    #[test]
    fn single_handler() {
        let router = Router::new().handler(sample_handler);
        assert_eq!(router.get_type(), "{ sample_handler: () => void }");
    }

    #[test]
    fn namespaced_single_handler() {
        let router = Router::namespace("ns").handler(sample_handler);
        assert_eq!(router.get_type(), "{ ns: { sample_handler: () => void } }");
    }

    #[test]
    fn multiple_handlers() {
        let router = Router::new()
            .handler(sample_handler)
            .handler(another_handler);
        assert_eq!(
            router.get_type(),
            "{ sample_handler: () => void, another_handler: () => void }"
        );
    }

    #[test]
    fn namespaced_multiple_handlers() {
        let router = Router::namespace("ns")
            .handler(sample_handler)
            .handler(another_handler);
        assert_eq!(
            router.get_type(),
            "{ ns: { sample_handler: () => void, another_handler: () => void } }"
        );
    }
}
