use super::Handler;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use ts_rs::TS;

impl<F, T1, Res> Handler<(T1,), Res> for F
where
    F: 'static + Fn(T1) -> Res + Clone + Send + Sync,
    T1: TS + DeserializeOwned,
    Res: TS + Serialize,
{
    fn call(&self, params: Value) -> Value {
        let params = serde_json::from_value::<(T1,)>(params).unwrap();

        let res = self(params.0);

        serde_json::to_value(res).unwrap()
    }

    fn get_parameter_types(&self) -> Vec<(String, String)> {
        vec![("p1".to_string(), T1::name())]
    }
}

impl<F, T1, T2, T3, Res> Handler<(T1, T2, T3), Res> for F
where
    F: 'static + Fn(T1, T2, T3) -> Res + Clone + Send + Sync,
    T1: TS + DeserializeOwned,
    T2: TS + DeserializeOwned,
    T3: TS + DeserializeOwned,
    Res: TS + Serialize,
{
    fn call(&self, params: Value) -> Value {
        let params = serde_json::from_value::<(T1, T2, T3)>(params).unwrap();

        let res = self(params.0, params.1, params.2);

        serde_json::to_value(res).unwrap()
    }

    fn get_parameter_types(&self) -> Vec<(String, String)> {
        vec![
            ("p1".to_string(), T1::name()),
            ("p2".to_string(), T2::name()),
            ("p3".to_string(), T3::name()),
        ]
    }
}
