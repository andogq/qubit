use qubit_macros::handler;

#[handler(query, name = "other_handler", name = "different_name")]
async fn my_handler() {}

fn main() {}
