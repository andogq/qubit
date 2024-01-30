pub mod handler;
pub mod server;

type RegisterHandlerFn = fn(RpcModule<()>) -> RpcModule<()>;

pub struct Router(Vec<RegisterHandlerFn>);

impl Router {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn handler(mut self, register: RegisterHandlerFn) -> Self {
        self.0.push(register);

        self
    }
}

use jsonrpsee::RpcModule;
pub use rs_ts_api_macros::*;
