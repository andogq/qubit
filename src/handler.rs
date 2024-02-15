use std::collections::BTreeMap;

use crate::rpc_builder::RpcBuilder;

#[derive(Debug)]
pub struct HandlerType {
    pub name: String,
    pub signature: String,
}

pub trait Handler {
    fn register(rpc_builder: RpcBuilder) -> RpcBuilder;

    fn get_type() -> HandlerType;

    fn add_dependencies(dependencies: &mut BTreeMap<String, String>);
}
