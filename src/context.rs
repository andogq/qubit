pub trait Context<AppCtx>
where
    Self: Sized,
{
    fn from_app_ctx(ctx: AppCtx) -> Option<Self>;
}

impl<Ctx> Context<Ctx> for Ctx {
    fn from_app_ctx(ctx: Ctx) -> Option<Self> {
        Some(ctx)
    }
}
