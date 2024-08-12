//! Handlers appear to be regular functions, however [`qubit_macros::handler`] expands them into
//! structs that implement the [`Handler`] trait. This expansion assists with both the run-time
//! [`crate::Router`] type generation, as well as other ergonomic features like parameter
//! deserialization from requests.
//!
//! There are two primary features that a handler must implement:
//!
//! - Normalisation and registration: The handlers must register themselves against a
//!   [`RpcBuilder`] instance in a uniform manner, so any parameters for this handler must be
//!   transformed from the parameters provided by the server.
//!
//! - Type specification: The handlers must emit both the signature of the handler
//!   ([`Handler::get_type`]), as well as any dependencies that they rely on
//!   ([`Handler::add_dependencies`]).
//!
//! # Handler Erasure
//!
//! In an effort to cut down on dynamic dispatch, [`HandlerCallbacks`] is a grab-bag of function
//! pointers to the methods of [`Handler`]. This is possible since none of these methods reference
//! `self`. This is what is actually stored on [`crate::Router`].

use std::path::Path;

use ts_rs::{Dependency, ExportError};

use crate::{builder::RpcBuilder, util::QubitType, HandlerType};

/// Handlers run for specific RPC requests. This trait will automatically be implemented if the
/// [`crate::handler`] macro is attached to a function containing a handler implementation.
pub trait Handler<AppCtx> {
    /// Register this handler against the provided RPC builder.
    fn register(rpc_builder: RpcBuilder<AppCtx>) -> RpcBuilder<AppCtx>;

    /// Get the type of this handler, to generate the client.
    fn get_type() -> HandlerType;

    fn export_all_dependencies_to(out_dir: &Path) -> Result<Vec<Dependency>, ExportError>;

    /// Provide a list of Qubit types that this handler relies on.
    fn qubit_types() -> Vec<QubitType>;
}

/// Wrapper struct to assist with erasure of concrete [`Handler`] type. Contains function pointers
/// to all of the implementations required to process the handler, allowing different handler types
/// to be contained together.
#[derive(Clone)]
pub(crate) struct HandlerCallbacks<Ctx> {
    /// Function pointer to the register implementation for the handler, which will register it
    /// against an RPC builder.
    pub register: fn(RpcBuilder<Ctx>) -> RpcBuilder<Ctx>,

    /// Function pointer to the implementation which will return the [`HandlerType`] for this
    /// handler.
    pub get_type: fn() -> HandlerType,

    pub export_all_dependencies_to: fn(&Path) -> Result<Vec<Dependency>, ExportError>,
    pub qubit_types: fn() -> Vec<QubitType>,
}

impl<Ctx> HandlerCallbacks<Ctx>
where
    Ctx: 'static + Send + Sync + Clone,
{
    /// Automatically implement the creation of [`HandlerCallbacks`] for anything that implements
    /// [`Handler`]. This is possible since the trait only contains static methods, which can simply
    /// be expressed as function pointers.
    pub fn from_handler<H: Handler<Ctx>>(_handler: H) -> Self {
        Self {
            register: H::register,
            get_type: H::get_type,
            export_all_dependencies_to: H::export_all_dependencies_to,
            qubit_types: H::qubit_types,
        }
    }
}
