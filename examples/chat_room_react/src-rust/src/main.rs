use std::net::SocketAddr;

use futures::Stream;
use manager::{ChatMessage, Client, Manager};
use qubit::{handler, Router};
use rand::{thread_rng, Rng};

mod manager;

#[derive(Clone)]
struct Ctx {
    client: Client,
    name: char,
}

#[handler]
async fn get_name(ctx: Ctx) -> char {
    ctx.name
}

#[handler(subscription)]
async fn list_online(ctx: Ctx) -> impl Stream<Item = Vec<char>> {
    ctx.client.stream_online().await
}

#[handler(subscription)]
async fn list_messages(ctx: Ctx) -> impl Stream<Item = Vec<ChatMessage>> {
    ctx.client.stream_messages().await
}

#[tokio::main]
async fn main() {
    // Construct the qubit router
    let router = Router::new()
        .handler(get_name)
        .handler(list_online)
        .handler(list_messages);

    // Save the type
    router.write_type_to_file("../src/bindings.ts");
    println!("Successfully wrote server types to `./bindings.ts`");

    // Create service and handle
    let client = Manager::start();
    let (qubit_service, qubit_handle) = router.to_service(move |_| {
        let client = client.clone();
        let name = random_emoji();
        async move {
            client.join(name).await;
            Ctx { client, name }
        }
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

fn random_emoji() -> char {
    char::from_u32(thread_rng().gen_range(0x1F600..0x1F64F)).unwrap_or('🦀')
}
