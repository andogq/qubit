use qubit_macros::handler;

#[handler(query, subscription)]
async fn my_handler() {}

fn main() {}
