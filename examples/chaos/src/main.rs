use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use futures::{stream, Stream, StreamExt};
use qubit::*;

use axum::routing::get;
use serde::{Deserialize, Serialize};

#[derive(ts_rs::TS, Clone, Serialize, Deserialize, Debug, ExportType)]
pub struct Metadata {
    param_a: String,
    param_b: u32,
    param_c: bool,

    more_metadata: Option<Box<Metadata>>,
}

#[derive(ts_rs::TS, Clone, Serialize, Deserialize, Debug, ExportType)]
pub struct User {
    name: String,
    email: String,
    age: u32,

    metadata: Metadata,
}

#[derive(ts_rs::TS, Clone, Serialize, Deserialize, Debug, ExportType)]
pub struct Test {
    a: usize,
    b: bool,
}

#[derive(Clone, Default)]
pub struct AppCtx {
    database: bool,
    log: String,

    count: Arc<AtomicUsize>,
}

mod user {
    use super::*;

    #[derive(Clone)]
    pub struct UserCtx {
        app_ctx: AppCtx,
        user: u32,
    }

    impl FromContext<AppCtx> for UserCtx {
        fn from_app_ctx(ctx: AppCtx) -> Result<Self, RpcError> {
            Ok(UserCtx {
                app_ctx: ctx,
                user: 0,
            })
        }
    }

    pub fn create_router() -> Router<AppCtx> {
        Router::new()
            .handler(someHandler)
            .handler(create)
            .handler(list)
            .handler(nested::asdf)
    }

    #[handler(name = "someHandler")]
    async fn get(_ctx: UserCtx, _id: String) -> User {
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

    mod nested {
        use super::*;

        #[handler]
        pub async fn asdf(_ctx: UserCtx) {
            todo!()
        }
    }

    #[handler]
    async fn create(_ctx: UserCtx, name: String, email: String, age: u32) -> User {
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

    #[handler]
    async fn list(_ctx: AppCtx) -> Vec<Test> {
        todo!()
    }
}

struct CountCtx {
    count: Arc<AtomicUsize>,
}

impl FromContext<AppCtx> for CountCtx {
    fn from_app_ctx(ctx: AppCtx) -> Result<Self, RpcError> {
        Ok(Self {
            count: ctx.count.clone(),
        })
    }
}

#[handler]
async fn count(ctx: CountCtx) -> usize {
    ctx.count.fetch_add(1, Ordering::Relaxed)
}

#[handler(subscription)]
async fn countdown(_ctx: AppCtx, min: usize, max: usize) -> impl Stream<Item = usize> {
    stream::iter(min..=max).then(|n| async move {
        tokio::time::sleep(Duration::from_secs(1)).await;

        n
    })
}

#[handler]
async fn version(_ctx: AppCtx) -> String {
    "v1.0.0".to_string()
}

#[handler]
async fn array(_ctx: AppCtx) -> Vec<String> {
    vec!["a".to_string(), "b".to_string(), "c".to_string()]
}

#[tokio::main]
async fn main() {
    // Build up the router
    let app = Router::new()
        .handler(version)
        .handler(count)
        .handler(countdown)
        .handler(array)
        .nest("user", user::create_router());

    // Save the router's bindings
    app.write_type_to_file("./bindings.ts");

    // Set up the context for the app
    let ctx = AppCtx::default();

    // Create a service and handle for the app
    let (app_service, app_handle) = app.to_service(move |_| ctx.clone());

    // Set up the axum router
    let router = axum::Router::<()>::new()
        .route("/", get(|| async { "working" }))
        .nest_service("/rpc", app_service);

    // Start the server
    hyper::Server::bind(&SocketAddr::from(([127, 0, 0, 1], 9944)))
        .serve(router.into_make_service())
        .await
        .unwrap();

    // Once the server has stopped, ensure that the app is shutdown
    app_handle.stop().unwrap();
}
