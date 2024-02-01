use std::net::SocketAddr;

use rs_ts_api::*;

use axum::routing::get;
use jsonrpsee::server::stop_channel;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(TS, Serialize, Deserialize, Debug)]
#[ts(export)]
pub struct Metadata {
    param_a: String,
    param_b: u32,
    param_c: bool,
}

#[derive(TS, Serialize, Deserialize, Debug)]
#[ts(export)]
/// Test doc
pub struct User {
    name: String,
    email: String,
    age: u32,

    metadata: Metadata,
}

mod user {
    use super::*;

    pub fn create_router() -> Router {
        Router::new().handler(get).handler(create)
    }

    #[handler]
    async fn get(_id: String) -> User {
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

    #[handler]
    async fn create(name: String, email: String, age: u32) -> User {
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
}

#[handler]
async fn version(_a: ()) -> String {
    "v1.0.0".to_string()
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .handler(version)
        .nest("user", user::create_router());

    println!("{}", app.get_type());

    let (stop_handle, server_handle) = stop_channel();

    let router = axum::Router::<()>::new()
        .route("/", get(|| async { "working" }))
        .nest_service("/rpc", app.create_service(stop_handle));

    hyper::Server::bind(&SocketAddr::from(([127, 0, 0, 1], 9944)))
        .serve(router.into_make_service())
        .await
        .unwrap();

    server_handle.stop().unwrap();
}
