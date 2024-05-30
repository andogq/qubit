use std::net::SocketAddr;

use futures::{stream, Stream};
use qubit::{handler, Router};

use crate::ctx::Ctx;

mod ctx;

// Simple handler, with no parameters from the client and no return values.
#[handler]
async fn increment(ctx: Ctx) {
    ctx.increment();
}

// Another simple handler.
#[handler]
async fn decrement(ctx: Ctx) {
    ctx.decrement();
}

// Handler that takes a parameter from the client.
#[handler]
async fn add(ctx: Ctx, n: i32) {
    ctx.add(n);
}

// Handler that returns a value to the client.
#[handler]
async fn get(ctx: Ctx) -> i32 {
    ctx.get()
}

// Handler that sets up a subscription, to continually stream data to the client.
#[handler(subscription)]
async fn countdown(ctx: Ctx) -> impl Stream<Item = i32> {
    stream::iter((0..=ctx.get()).rev())
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
    router.write_type_to_file("./bindings.ts");
    println!("Successfully write server type to `./bindings.ts`");

    // Create initial ctx
    let ctx = Ctx::default();

    // Create service and handle
    let (qubit_service, qubit_handle) = router.to_service(move |_| {
        let ctx = ctx.clone();
        async { ctx }
    });

    // Nest into an Axum rouer
    let axum_router = axum::Router::<()>::new().nest_service("/rpc", qubit_service);

    // Start a Hyper server
    println!("Listening at 127.0.0.1:9944");
    hyper::Server::bind(&SocketAddr::from(([127, 0, 0, 1], 9944)))
        .serve(axum_router.into_make_service())
        .await
        .unwrap();

    // Shutdown Qubit
    qubit_handle.stop().unwrap();
}
