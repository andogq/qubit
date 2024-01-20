use serde_json::Value;

pub trait ErasedHandler {
    fn call(self: Box<Self>, params: Value) -> Value;
    fn clone_box(&self) -> Box<dyn ErasedHandler>;

    fn get_signature(&self) -> (Vec<(String, String)>, String);
}

pub struct MakeErasedHandler<H> {
    pub handler: H,
    pub do_call: fn(H, Value) -> Value,
    pub get_signature: fn(&H) -> (Vec<(String, String)>, String),
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
            get_signature: self.get_signature,
        })
    }

    fn get_signature(&self) -> (Vec<(String, String)>, String) {
        (self.get_signature)(&self.handler)
    }
}
