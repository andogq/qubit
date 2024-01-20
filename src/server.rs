use std::rc::Rc;

use matchit::Router;
use serde_json::Value;
use ts_rs::TS;

use crate::handler::{ErasedHandler, Handler, MakeErasedHandler};

pub struct Server {
    /// Keep all handlers in a Vec in-order to infer signatuers for server
    signature_routes: Vec<(String, Rc<Box<dyn ErasedHandler>>)>,

    /// Only allow router to store references to handlers
    router: Router<Rc<Box<dyn ErasedHandler>>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            signature_routes: Vec::new(),
            router: Router::new(),
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
        let handler = Rc::new(Box::new(MakeErasedHandler {
            handler,
            do_call: |handler, params| handler.call(params),
            get_signature: |handler| (handler.get_parameter_types(), handler.get_return_type()),
        }) as Box<dyn ErasedHandler>);

        self.signature_routes
            .push((route.as_ref().to_string(), handler.clone()));

        self.router
            .insert(route.as_ref().to_string(), handler)
            .unwrap();

        self
    }

    pub fn call(&self, route: impl AsRef<str>, parameters: Value) -> Value {
        self.router
            .at(route.as_ref())
            .unwrap()
            .value
            .clone_box()
            .call(parameters)
    }

    pub fn get_signatures(&self) -> String {
        let body = self
            .signature_routes
            .iter()
            .map(|(route, handler)| {
                let (parameter_types, return_type) = handler.get_signature();

                let signature = format!(
                    "{}: ({}) => {return_type}",
                    route,
                    parameter_types
                        .into_iter()
                        .map(|(param, ty)| format!("{param}: {ty}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );

                signature
            })
            .collect::<Vec<_>>();

        format!("{{ {} }}", body.join(", "))
    }
}
