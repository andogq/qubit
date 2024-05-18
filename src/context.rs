use crate::error::RpcError;

pub trait FromContext<AppCtx>
where
    Self: Sized,
{
    fn from_app_ctx(ctx: AppCtx) -> Result<Self, RpcError>;
}

impl<Ctx> FromContext<Ctx> for Ctx {
    fn from_app_ctx(ctx: Ctx) -> Result<Self, RpcError> {
        Ok(ctx)
    }
}
