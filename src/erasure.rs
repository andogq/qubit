use std::collections::HashMap;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::{Metadata, User};

pub trait Handler<Params, Return>: Clone + 'static {
    fn call(self, params: Value) -> Value;
}

impl<F, T1, Res> Handler<(T1,), Res> for F
where
    F: Fn(T1) -> Res + Clone + 'static,
    T1: TS + DeserializeOwned,
    Res: TS + Serialize,
{
    fn call(self, params: Value) -> Value {
        let params = serde_json::from_value::<(T1,)>(params).unwrap();

        let res = self(params.0);

        serde_json::to_value(res).unwrap()
    }
}

impl<F, T1, T2, T3, Res> Handler<(T1, T2, T3), Res> for F
where
    F: Fn(T1, T2, T3) -> Res + Clone + 'static,
    T1: TS + DeserializeOwned,
    T2: TS + DeserializeOwned,
    T3: TS + DeserializeOwned,
    Res: TS + Serialize,
{
    fn call(self, params: Value) -> Value {
        let params = serde_json::from_value::<(T1, T2, T3)>(params).unwrap();

        let res = self(params.0, params.1, params.2);

        serde_json::to_value(res).unwrap()
    }
}

trait ErasedHandler {
    fn clone_box(&self) -> Box<dyn ErasedHandler>;

    fn call(self: Box<Self>, params: Value) -> Value;
}

struct MakeErasedHandler<H> {
    pub handler: H,
    pub do_call: fn(H, Value) -> Value,
}

impl<H> ErasedHandler for MakeErasedHandler<H>
where
    H: 'static + Clone,
{
    fn call(self: Box<Self>, params: Value) -> Value {
        (self.do_call)(self.handler, params)
    }

    fn clone_box(&self) -> Box<dyn ErasedHandler> {
        Box::new(Self {
            handler: self.handler.clone(),
            do_call: self.do_call,
        })
    }
}

// -------

pub struct Server {
    routes: HashMap<String, Box<dyn ErasedHandler>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
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

pub fn create_server() -> Server {
    Server::new()
        .add("get", get_user)
        .add("create", create_user)
}

fn get_user(id: String) -> User {
    println!("get user");

    User {
        name: "some user".to_string(),
        email: "email@example.com".to_string(),
        age: 100,
        metadata: Metadata {
            param_a: String::new(),
            param_b: 123,
            param_c: true,
        },
    }
}

fn create_user(name: String, email: String, age: u32) -> User {
    todo!();
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    #[test]
    fn routing() {
        let server = create_server();

        dbg!(server.call("get", json!(["test_id"])));
    }
}
