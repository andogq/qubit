use serde_json::Value;

pub trait ErasedHandler {
    fn clone_box(&self) -> Box<dyn ErasedHandler>;

    fn call(self: Box<Self>, params: Value) -> Value;
}

pub struct MakeErasedHandler<H> {
    pub handler: H,
    pub do_call: fn(H, Value) -> Value,
}

impl<H> ErasedHandler for MakeErasedHandler<H>
where
    H: 'static + Clone,
{
    fn call(self: Box<Self>, params: Value) -> Value {
        (self.do_call)(self.handler, params)
    }

    fn clone_box(&self) -> Box<dyn ErasedHandler> {
        Box::new(Self {
            handler: self.handler.clone(),
            do_call: self.do_call,
        })
    }
}
