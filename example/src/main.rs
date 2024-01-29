mod handler;

use std::net::SocketAddr;

use rs_ts_api::*;

use axum::{routing::get, Router};
use jsonrpsee::server::stop_channel;
use serde::{Deserialize, Serialize};
use server::Server;
use ts_rs::TS;

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

fn get_user(_id: String) -> User {
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
    println!("creating user: {name}");

    User {
        name,
        email,
        age,
        metadata: Metadata {
            param_a: String::new(),
            param_b: 123,
            param_c: true,
        },
    }
}

#[tokio::main]
async fn main() {
    let server = create_server();
    dbg!(server.get_signatures());

    let (stop_handle, server_handle) = stop_channel();

    let router = Router::<()>::new()
        .route("/", get(|| async { "working" }))
        .nest_service("/rpc", server.create_service(stop_handle));

    hyper::Server::bind(&SocketAddr::from(([127, 0, 0, 1], 9944)))
        .serve(router.into_make_service())
        .await
        .unwrap();

    server_handle.stop().unwrap();
}
