use std::collections::BTreeMap;

use serde_json::Value;

use crate::handler::{ErasedHandler, Handler, MakeErasedHandler};

pub struct Server {
    routes: BTreeMap<String, Box<dyn ErasedHandler>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            routes: BTreeMap::new(),
        }
    }

    pub fn add<Params, Return>(
        mut self,
        route: impl AsRef<str>,
        handler: impl Handler<Params, Return> + 'static,
    ) -> Self {
        self.routes.insert(
            route.as_ref().to_string(),
            Box::new(MakeErasedHandler {
                handler,
                do_call: |handler, params| handler.call(params),
            }),
        );

        self
    }

    pub fn call(&self, route: impl AsRef<str>, parameters: Value) -> Value {
        self.routes
            .get(route.as_ref())
            .unwrap()
            .clone_box()
            .call(parameters)
    }
}
