use qubit_macros::handler;

#[handler(query, name = "other_handler", name = "other_handler")]
async fn my_handler() {}

fn main() {}
