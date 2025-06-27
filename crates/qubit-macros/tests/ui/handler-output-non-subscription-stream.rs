use qubit_macros::handler;

#[handler(query)]
async fn my_handler() -> impl Stream<Item = u32> {}

fn main() {}
