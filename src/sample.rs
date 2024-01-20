use std::collections::HashMap;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::User;

fn get_user(id: String) -> User {
    todo!();
}

fn create_user(name: String, email: String, age: u32) -> User {
    todo!();
}

pub struct Server {
    routes: HashMap<String, Box<dyn ErasedHandlerFunction>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    pub fn add(
        mut self,
        route: impl AsRef<str>,
        handler: impl HandlerFunction<dyn TS, dyn TS>,
    ) -> Self {
        self.routes
            .insert(route.as_ref().to_string(), Box::new(handler));

        self
    }
}

pub fn create_server() {
    Server::new()
        .add("get", get_user)
        .add("create", create_user);
}

trait HandlerFunction<Params, Return> {
    fn call(&self, params: Value) -> Value;
}

impl<F, T1, Res> HandlerFunction<(T1,), Res> for F
where
    T1: TS + DeserializeOwned,
    Res: TS + Serialize,
    F: Fn(T1) -> Res,
{
    fn call(&self, params: Value) -> Value {
        let params = serde_json::from_value::<(T1,)>(params).unwrap();

        serde_json::to_value(self(params.0))
    }
}

trait HandlerParams {}

// ------ Erasure stuff -------

trait ErasedHandlerFunction {
    fn call(&self, params: Value) -> Value;
}

pub struct MakeErasedHandlerFunction<H> {
    pub handler: H,
}

impl<H> ErasedHandlerFunction for MakeErasedHandlerFunction<H> {
    fn call(&self, params: Value) -> Value {
        self.handler(params)
    }
}
