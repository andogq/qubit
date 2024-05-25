//! Type generation specific functionality. There is no real need for this to be directly used,
//! [`qubit_macros::handler`] should handle it all.

mod export_type;

pub use export_type::*;

/// Components used to construct the client type for this handler.
#[derive(Debug)]
pub struct HandlerType {
    /// Unique name of the handler. This will automatically be namespaced as appropriate when the
    /// attached router is nested.
    pub name: String,

    /// Signature of this handler.
    pub signature: String,
}
