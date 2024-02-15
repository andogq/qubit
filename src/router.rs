use std::{collections::BTreeMap, fs, path::Path};

use jsonrpsee::{server::StopHandle, RpcModule};

use crate::{
    handler::{Handler, HandlerCallbacks},
    rpc_builder::RpcBuilder,
    server::ServerService,
};

/// Router for the RPC server. Can have different handlers attached to it, as well as nested
/// routers in order to create a hierarchy. It is also capable of generating its own type, suitable
/// for consumption by a TypeScript client.
pub struct Router {
    nested_routers: Vec<(&'static str, Router)>,
    handlers: Vec<HandlerCallbacks>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            nested_routers: Vec::new(),
            handlers: Vec::new(),
        }
    }

    pub fn handler<H: Handler>(mut self, handler: H) -> Self {
        self.handlers.push(handler.into());

        self
    }

    pub fn nest(mut self, namespace: &'static str, router: Router) -> Self {
        self.nested_routers.push((namespace, router));

        self
    }

    pub fn add_dependencies(&self, dependencies: &mut BTreeMap<String, String>) {
        // Add all handler dependencies
        self.handlers
            .iter()
            .for_each(|handler| (handler.add_dependencies)(dependencies));

        // Add dependencies for nested routers
        self.nested_routers
            .iter()
            .for_each(|(_, router)| router.add_dependencies(dependencies));
    }

    pub fn get_type(&self) -> String {
        let mut handlers = self
            .handlers
            .iter()
            .map(|handler| (handler.get_type)())
            .map(|handler_type| format!("{}: {}", handler_type.name, handler_type.signature))
            .collect::<Vec<_>>();

        self.nested_routers
            .iter()
            .map(|(namespace, router)| (namespace, router.get_type()))
            .for_each(|(namespace, router_type)| {
                handlers.push(format!("{namespace}: {router_type}"));
            });

        // Generate the router type
        let router_type = format!("{{ {} }}", handlers.join(", "));

        router_type
    }

    pub fn write_type_to_file(&self, path: impl AsRef<Path>) {
        let mut dependencies = BTreeMap::new();
        self.add_dependencies(&mut dependencies);
        let dependencies = dependencies
            .into_iter()
            .map(|(name, ty)| format!("type {name} = {ty};"))
            .collect::<Vec<_>>()
            .join("\n");

        let router = self.get_type();
        let router = format!("export type Server = {router};");

        fs::write(path, format!("{dependencies}\n{router}")).unwrap();
    }

    pub fn build_rpc_module(self, namespace: Option<&'static str>) -> RpcModule<()> {
        let mut rpc_module = self
            .handlers
            .into_iter()
            .fold(
                RpcBuilder::with_namespace(namespace),
                |rpc_builder, handler| (handler.register)(rpc_builder),
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
