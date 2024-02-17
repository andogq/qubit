use jsonrpsee::types::ErrorObjectOwned;

pub trait Context<AppCtx>
where
    Self: Sized,
{
    fn from_app_ctx(ctx: AppCtx) -> Result<Self, ErrorObjectOwned>;
}

impl<Ctx> Context<Ctx> for Ctx {
    fn from_app_ctx(ctx: Ctx) -> Result<Self, ErrorObjectOwned> {
        Ok(ctx)
    }
}
