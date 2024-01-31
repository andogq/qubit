use jsonrpsee::RpcModule;
pub use rs_ts_api_macros::*;

pub mod handler;
pub mod server;

type RegisterHandlerFn = fn(RpcModule<()>) -> RpcModule<()>;

pub trait NewHandler {
    fn register(router: jsonrpsee::RpcModule<()>) -> jsonrpsee::RpcModule<()>;

    fn get_type() -> String;
}

pub struct Router {
    name: Option<String>,
    handlers: Vec<(fn() -> String, RegisterHandlerFn)>,
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

    pub fn handler<H: NewHandler>(mut self, _: H) -> Self {
        self.handlers.push((H::get_type, H::register));

        self
    }

    pub fn get_type(&self) -> String {
        let handlers = self
            .handlers
            .iter()
            .map(|(get_signature, _)| get_signature())
            .collect::<Vec<_>>()
            .join(", ");

        let mut signature = format!("{{ {handlers} }}");

        if let Some(name) = &self.name {
            signature = format!("{{ {name}: {signature} }}");
        }

        signature
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[allow(non_camel_case_types)]
    struct sample_handler;
    impl NewHandler for sample_handler {
        fn register(_router: jsonrpsee::RpcModule<()>) -> jsonrpsee::RpcModule<()> {
            todo!()
        }

        fn get_type() -> String {
            "fn_type".to_string()
        }
    }

    #[allow(non_camel_case_types)]
    struct another_handler;
    impl NewHandler for another_handler {
        fn register(_router: jsonrpsee::RpcModule<()>) -> jsonrpsee::RpcModule<()> {
            todo!()
        }

        fn get_type() -> String {
            "another_fn_type".to_string()
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
        assert_eq!(router.get_type(), "{ fn_type }");
    }

    #[test]
    fn namespaced_single_handler() {
        let router = Router::namespace("ns").handler(sample_handler);
        assert_eq!(router.get_type(), "{ ns: { fn_type } }");
    }

    #[test]
    fn multiple_handlers() {
        let router = Router::new()
            .handler(sample_handler)
            .handler(another_handler);
        assert_eq!(router.get_type(), "{ fn_type, another_fn_type }");
    }

    #[test]
    fn namespaced_multiple_handlers() {
        let router = Router::namespace("ns")
            .handler(sample_handler)
            .handler(another_handler);
        assert_eq!(router.get_type(), "{ ns: { fn_type, another_fn_type } }");
    }
}
