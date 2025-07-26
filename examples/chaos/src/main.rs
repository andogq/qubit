use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use futures::{Stream, StreamExt, stream};
use qubit::*;

use axum::routing::get;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[ts(optional_fields)]
pub struct Metadata {
    param_a: String,
    param_b: u32,
    param_c: bool,

    more_metadata: Option<Box<Metadata>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[ts]
pub struct User {
    name: String,
    email: String,
    age: u32,

    metadata: Metadata,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[ts]
pub struct Test {
    a: usize,
    b: bool,
}

#[derive(Clone, Default)]
#[allow(dead_code)]
pub struct AppCtx {
    database: bool,
    log: String,

    count: Arc<AtomicUsize>,
}

mod user {
    use super::*;

    #[derive(Clone)]
    #[allow(dead_code)]
    pub struct UserCtx {
        app_ctx: AppCtx,
        user: u32,
    }

    impl FromRequestExtensions<AppCtx> for UserCtx {
        async fn from_request_extensions(
            ctx: AppCtx,
            _extensions: Extensions,
        ) -> Result<Self, RpcError> {
            Ok(UserCtx {
                app_ctx: ctx,
                user: 0,
            })
        }
    }

    pub fn create_router() -> Router<AppCtx> {
        Router::new()
            .handler(get)
            .handler(create)
            .handler(list)
            .handler(nested::asdf)
    }

    #[handler(query, name = "someHandler")]
    async fn get(_ctx: AppCtx, _id: String) -> User {
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

        #[handler(query)]
        pub async fn asdf() {
            todo!()
        }
    }

    #[handler(mutation)]
    async fn create(_ctx: AppCtx, name: String, email: String, age: u32) -> User {
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

    #[handler(query)]
    async fn list() -> Vec<Test> {
        todo!()
    }
}

struct CountCtx {
    count: Arc<AtomicUsize>,
}

impl FromRequestExtensions<AppCtx> for CountCtx {
    async fn from_request_extensions(
        ctx: AppCtx,
        _extensions: Extensions,
    ) -> Result<Self, RpcError> {
        Ok(Self {
            count: ctx.count.clone(),
        })
    }
}

#[handler(mutation)]
async fn count(ctx: CountCtx) -> usize {
    ctx.count.fetch_add(1, Ordering::Relaxed)
}

#[handler(subscription)]
async fn countdown(_ctx: CountCtx, min: usize, max: usize) -> impl Stream<Item = usize> {
    stream::iter(min..=max).then(|n| async move {
        tokio::time::sleep(Duration::from_secs(1)).await;

        n
    })
}

#[handler(query)]
async fn version() -> String {
    "v1.0.0".to_string()
}

#[handler(query)]
async fn array() -> Vec<String> {
    vec!["a".to_string(), "b".to_string(), "c".to_string()]
}

#[derive(Clone, Serialize)]
#[ts]
struct UniqueType {
    value: usize,
}

#[handler(query)]
async fn array_type() -> Vec<UniqueType> {
    vec![]
}

#[derive(Clone, Serialize)]
#[ts]
struct NestedStruct {
    a: f32,
    b: bool,
}

#[derive(Clone, Serialize)]
#[ts]
#[allow(dead_code)]
enum MyEnum {
    A,
    B(u8),
    C { field: u8 },
    D(NestedStruct),
}
#[handler(query)]
async fn enum_test() -> MyEnum {
    MyEnum::B(10)
}

#[tokio::main]
async fn main() {
    // Build up the router
    let app = Router::<AppCtx>::new()
        .handler(version)
        .handler(count)
        .handler(countdown)
        .handler(array)
        .handler(enum_test)
        .handler(array_type)
        .nest("user", user::create_router());

    // Save the router's bindings
    app.generate_type("./bindings.ts").unwrap();

    // Create a service and handle for the app
    let (app_service, app_handle) = app.into_service(AppCtx::default());

    // Set up the axum router
    let router = axum::Router::<()>::new()
        .route("/", get(|| async { "working" }))
        .nest_service("/rpc", app_service);

    // Start the server
    axum::serve(
        TcpListener::bind(&SocketAddr::from(([127, 0, 0, 1], 9944)))
            .await
            .unwrap(),
        router,
    )
    .await
    .unwrap();

    // Once the server has stopped, ensure that the app is shutdown
    app_handle.stop().unwrap();
}
