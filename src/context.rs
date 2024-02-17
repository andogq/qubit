use jsonrpsee::types::ErrorObjectOwned;

pub trait FromContext<AppCtx>
where
    Self: Sized,
{
    fn from_app_ctx(ctx: AppCtx) -> Result<Self, ErrorObjectOwned>;
}

impl<Ctx> FromContext<Ctx> for Ctx {
    fn from_app_ctx(ctx: Ctx) -> Result<Self, ErrorObjectOwned> {
        Ok(ctx)
    }
}
