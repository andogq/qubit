use http::Extensions;

use crate::RpcError;

/// Context can be built from request information by implementing the following trait. The
/// extensions are passed in from the request (see [`Extensions`]), which can be added using tower
/// middleware.
#[trait_variant::make(Send)]
pub trait FromRequestExtensions<Ctx>: Sized {
    /// Using the provided context and extensions, build a new extension.
    async fn from_request_extensions(ctx: Ctx, extensions: Extensions) -> Result<Self, RpcError>;
}

/// Blanket implementation for any type used as context, so that it can also fulfill this trait.
impl<Ctx> FromRequestExtensions<Ctx> for Ctx
where
    Ctx: Send,
{
    async fn from_request_extensions(ctx: Ctx, _extensions: Extensions) -> Result<Self, RpcError> {
        Ok(ctx)
    }
}
