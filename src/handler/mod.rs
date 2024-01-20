use serde_json::Value;

mod erasure;
mod impls;

pub use erasure::{ErasedHandler, MakeErasedHandler};

pub trait Handler<Params, Return>: Clone + 'static {
    fn call(self, params: Value) -> Value;
}
