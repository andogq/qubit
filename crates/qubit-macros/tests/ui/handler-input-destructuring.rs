use qubit_macros::handler;

#[handler(query)]
async fn my_handler(MyStruct { a, b }: MyStruct) {}

fn main() {}
