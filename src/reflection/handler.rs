//! Anything related to runtime reflection of handlers.
use std::{any::TypeId, collections::BTreeMap};

use lazy_static::lazy_static;
use linkme::distributed_slice;

use crate::QubitHandler;

/// The key to runtime handler reflection. In order to 'smuggle' information from the proc-macro
/// back into the runtime, information needs to be stored in the binary. [`linkme`] is used to
/// create a slice of function pointers, which can be added at compile time as the
/// [`crate::handler`] proc-macro is expanded.
#[distributed_slice]
pub static HANDLER_DEFINITIONS: [fn() -> (TypeId, HandlerMeta)];

lazy_static! {
    /// Runtime utility to easily lookup [`HandlerMeta`] for a [`TypeId`].
    static ref HANDLER_DEFINITIONS_MAP: BTreeMap<TypeId, HandlerMeta> = HANDLER_DEFINITIONS
        .into_iter()
        .map(|def_fn| def_fn())
        .collect();
}

/// Kind of the handler. This will correspond with the method the user must call from
/// TypeScript.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandlerKind {
    Query,
    Mutation,
    Subscription,
}

/// Static metadata associated with handler.
///
/// This should be generated with the [`handler`](crate::handler) macro.
#[derive(Clone, Debug)]
pub struct HandlerMeta {
    /// Kind of the handler.
    pub kind: HandlerKind,
    /// RPC name of the handler (this may differ from the name of the handler function).
    pub name: &'static str,
    /// Name of the parameters for this handler.
    pub param_names: &'static [&'static str],
}

impl HandlerMeta {
    /// Lookup the [`HandlerMeta`] of a handler. If the handler is not found (such as if it wasn't
    /// registered with [`crate::handler`], then this method will panic).
    pub(crate) fn of<F, Ctx, MSig>(#[allow(unused)] handler: &F) -> &'static Self
    where
        F: QubitHandler<Ctx, MSig>,
    {
        HANDLER_DEFINITIONS_MAP.get(&TypeId::of::<F>()).unwrap()
    }
}
