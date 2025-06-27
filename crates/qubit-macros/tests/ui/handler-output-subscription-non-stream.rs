use qubit_macros::handler;

#[handler(subscription)]
async fn my_handler() -> usize {}

fn main() {}
