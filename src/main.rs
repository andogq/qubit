use serde::{Deserialize, Serialize};
use ts_rs::TS;

mod erasure;
// mod sample;

#[derive(TS, Serialize, Deserialize)]
#[ts(export)]
pub struct Metadata {
    param_a: String,
    param_b: u32,
    param_c: bool,
}

#[derive(TS, Serialize, Deserialize)]
#[ts(export)]
/// Test doc
pub struct User {
    name: String,
    email: String,
    age: u32,

    metadata: Metadata,
}

// fn get_user(id: String) -> User {
//     todo!()
// }
//
// fn handler(f: impl TsFunction<impl TsParams, impl TS>) {
//     todo!()
// }
//
type Random = (String, u32);

fn main() {
    dbg!(Random::name());
}
//
// trait TsParams {
//     fn get_types() -> Vec<String>;
// }
//
// // impl<T1> TsParams for (T1)
// // where
// //     T1: TS,
// // {
// //     fn get_types() -> Vec<String> {
// //         vec![T1::name()]
// //     }
// // }
//
// impl<T1, T2> TsParams for (T1, T2)
// where
//     T1: TS,
//     T2: TS,
// {
//     fn get_types() -> Vec<String> {
//         vec![T1::name(), T2::name()]
//     }
// }
//
// impl<T1, T2, T3> TsParams for (T1, T2, T3)
// where
//     T1: TS,
//     T2: TS,
//     T3: TS,
// {
//     fn get_types() -> Vec<String> {
//         vec![T1::name(), T2::name(), T3::name()]
//     }
// }
//
// trait TsFunction<TParams, TReturn>
// where
//     TParams: TsParams,
//     TReturn: TS,
// {
// }
//
// impl<F, T1, T2, Res> TsFunction<(T1, T2), Res> for F
// where
//     F: Fn(T1, T2) -> Res,
//     T1: TS,
//     T2: TS,
//     Res: TS,
// {
// }
// impl<F, T1, T2, T3, Res> TsFunction<(T1, T2, T3), Res> for F
// where
//     F: Fn(T1, T2, T3) -> Res,
//     T1: TS,
//     T2: TS,
//     T3: TS,
//     Res: TS,
// {
// }
