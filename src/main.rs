use serde::{Deserialize, Serialize};
use serde_json::json;
use server::Server;
use ts_rs::TS;

mod handler;
mod server;

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

pub fn create_server() -> Server {
    Server::new()
        .add("get", get_user)
        .add("create", create_user)
}

fn get_user(id: String) -> User {
    println!("get user");

    User {
        name: "some user".to_string(),
        email: "email@example.com".to_string(),
        age: 100,
        metadata: Metadata {
            param_a: String::new(),
            param_b: 123,
            param_c: true,
        },
    }
}

fn create_user(name: String, email: String, age: u32) -> User {
    todo!();
}

fn main() {
    let server = create_server();

    dbg!(server.call("get", json!(["test_id"])));
}
