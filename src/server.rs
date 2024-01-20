use std::collections::BTreeMap;

use serde_json::Value;
use ts_rs::TS;

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
    ) -> Self
    where
        Params: TS,
        Return: TS,
    {
        self.routes.insert(
            route.as_ref().to_string(),
            Box::new(MakeErasedHandler {
                handler,
                do_call: |handler, params| handler.call(params),
                get_signature: |handler| (handler.get_parameter_types(), handler.get_return_type()),
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

    pub fn get_signatures(&self) -> Vec<String> {
        self.routes
            .iter()
            .map(|(route, handler)| {
                let (parameter_types, return_type) = handler.get_signature();

                let signature = format!(
                    "const {} = ({}) => {return_type};",
                    route,
                    parameter_types
                        .into_iter()
                        .map(|(param, ty)| format!("{param}: {ty}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );

                println!("{signature}");
                signature
            })
            .collect()
    }
}
