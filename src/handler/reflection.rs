//! Utilities for passing handlers and associated information at runtime.

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
///  This should be generated with the [`handler`](crate::handler) macro.
#[derive(Clone, Debug)]
pub struct HandlerMeta {
    /// Kind of the handler.
    pub kind: HandlerKind,
    /// RPC name of the handler (this may differ from the name of the handler function).
    pub name: &'static str,
    /// Name of the parameters for this handler.
    pub param_names: &'static [&'static str],
}
