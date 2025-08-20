use std::{net::SocketAddr, time::Duration};

use futures::{stream, Stream, StreamExt};
use qubit::{handler, Router, TypeScript};
use tokio::net::TcpListener;

use crate::ctx::Ctx;

mod ctx;

// Simple handler, with no parameters from the client and no return values.
#[handler(mutation)]
async fn increment(ctx: Ctx) {
    ctx.increment();
}

// Another simple handler.
#[handler(mutation)]
async fn decrement(ctx: Ctx) {
    ctx.decrement();
}

// Handler that takes a parameter from the client.
#[handler(mutation)]
async fn add(ctx: Ctx, n: i32) {
    ctx.add(n);
}

// Handler that returns a value to the client.
#[handler(query)]
async fn get(ctx: Ctx) -> i32 {
    ctx.get()
}

// Handler that sets up a subscription, to continually stream data to the client.
#[handler(subscription)]
async fn countdown(ctx: Ctx) -> impl Stream<Item = i32> {
    stream::iter((0..=ctx.get()).rev()).then(|item| async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        item
    })
}

#[tokio::main]
async fn main() {
    // Construct the qubit router
    let router = Router::new()
        .handler(increment)
        .handler(decrement)
        .handler(add)
        .handler(get)
        .handler(countdown);

    // Save the type
    router
        .as_codegen()
        .write_type("./bindings.ts", TypeScript::new())
        .unwrap();
    println!("Successfully write bindings to `./bindings.ts`");

    // Create service and handle
    let (qubit_service, qubit_handle) = router.as_rpc(Ctx::default()).into_service();

    // Nest into an Axum rouer
    let axum_router = axum::Router::<()>::new().nest_service("/rpc", qubit_service);

    // Start a Hyper server
    println!("Listening at 127.0.0.1:9944");
    axum::serve(
        TcpListener::bind(&SocketAddr::from(([127, 0, 0, 1], 9944)))
            .await
            .unwrap(),
        axum_router,
    )
    .await
    .unwrap();

    // Shutdown Qubit
    qubit_handle.stop().unwrap();
}
