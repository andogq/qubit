use std::collections::BTreeMap;

use crate::rpc_builder::RpcBuilder;

/// Components used to construct the client type for this handler.
#[derive(Debug)]
pub struct HandlerType {
    /// Unique name of the handler. This will automatically be namespaced as appropriate when the
    /// attached router is nested.
    pub name: String,

    /// Signature of this handler.
    pub signature: String,
}

/// Handlers run for specific RPC requests. This trait will automatically be implemented if the
/// [`crate::handler`] macro is attached to a function containing a handler implementation.
pub trait Handler {
    /// Register this handler against the provided RPC builder.
    fn register(rpc_builder: RpcBuilder) -> RpcBuilder;

    /// Get the type of this handler, to generate the client.
    fn get_type() -> HandlerType;

    /// Get any dependencies required to use this [`HandlerType`] in the client.
    fn add_dependencies(dependencies: &mut BTreeMap<String, String>);
}

/// Wrapper struct to assist with erasure of concrete [`Handler`] type. Contains function pointers
/// to all of the implementations required to process the handler, allowing different handler types
/// to be contained together.
pub(crate) struct HandlerCallbacks {
    /// Function pointer to the register implementation for the handler, which will register it
    /// against an RPC builder.
    pub register: fn(RpcBuilder) -> RpcBuilder,

    /// Function pointer to the implementation which will return the [`HandlerType`] for this
    /// handler.
    pub get_type: fn() -> HandlerType,

    /// Function pointer to the implementation that will add any type dependencies for the handler
    /// to the provided collection.
    pub add_dependencies: fn(&mut BTreeMap<String, String>),
}

/// Automatically implement the creation of [`HandlerCallbacks`] for anything that implements
/// [`Handler`]. This is possible since the trait only contains static methods, which can simply be
/// expressed as function pointers.
impl<H: Handler> From<H> for HandlerCallbacks {
    fn from(_handler: H) -> Self {
        Self {
            register: H::register,
            get_type: H::get_type,
            add_dependencies: H::add_dependencies,
        }
    }
}
