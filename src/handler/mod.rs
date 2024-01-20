use serde_json::Value;

mod erasure;
mod impls;

#[allow(unused_imports)]
pub use erasure::{ErasedHandler, MakeErasedHandler};

use ts_rs::TS;

pub trait Handler<Params, Return>: 'static + Clone + Send + Sync
where
    Params: TS,
    Return: TS,
{
    fn call(&self, params: Value) -> Value;

    fn get_parameter_types(&self) -> Vec<(String, String)>;
    fn get_return_type(&self) -> String {
        Return::name()
    }
}
