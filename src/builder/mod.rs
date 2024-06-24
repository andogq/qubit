//! Components used to 'build' the [`crate::Router`]. This module mostly contains components that
//! will primarily be used by [`qubit_macros`], directly using them probably isn't a great idea.

pub mod handler;
mod rpc_builder;
pub mod ty;

pub use handler::Handler;
pub(crate) use handler::HandlerCallbacks;
pub use rpc_builder::RpcBuilder;
pub use ty::*;

pub use jsonrpsee::types::ErrorObject;
pub use jsonrpsee::IntoResponse;
