use std::{collections::BTreeMap, net::SocketAddr};

use rs_ts_api::*;

use axum::routing::get;
use jsonrpsee::server::stop_channel;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug)]
#[exported_type]
pub struct Metadata {
    param_a: String,
    param_b: u32,
    param_c: bool,

    more_metadata: Option<Box<Metadata>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[exported_type]
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

                more_metadata: None,
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

                more_metadata: None,
            },
        }
    }
}

#[handler]
async fn version() -> String {
    "v1.0.0".to_string()
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .handler(version)
        .nest("user", user::create_router());

    let (stop_handle, server_handle) = stop_channel();

    let mut dependencies = BTreeMap::new();
    app.add_dependencies(&mut dependencies);
    dbg!(dependencies);

    dbg!(app.get_type());

    let router = axum::Router::<()>::new()
        .route("/", get(|| async { "working" }))
        .nest_service("/rpc", app.create_service(stop_handle));

    hyper::Server::bind(&SocketAddr::from(([127, 0, 0, 1], 9944)))
        .serve(router.into_make_service())
        .await
        .unwrap();

    server_handle.stop().unwrap();
}
