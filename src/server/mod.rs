mod error;
mod router;

pub use error::*;
pub use router::{Router, ServerHandle};

/// Router context variation that can derived from `Ctx`.
#[trait_variant::make(FromContext: Send)]
pub trait LocalFromContext<Ctx>
where
    Self: Sized,
{
    /// Create a new instance from the provided context.
    ///
    /// This is falliable, so any errors must produce a [`RpcError`], which will be returned to the
    /// client.
    async fn from_app_ctx(ctx: Ctx) -> Result<Self, RpcError>;
}

impl<Ctx: Send> FromContext<Ctx> for Ctx {
    async fn from_app_ctx(ctx: Ctx) -> Result<Self, RpcError> {
        Ok(ctx)
    }
}
