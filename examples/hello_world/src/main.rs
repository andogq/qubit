use std::net::SocketAddr;

use qubit::{handler, Router};

#[handler]
async fn hello_world(_ctx: ()) -> String {
    "Hello, world!".to_string()
}

#[tokio::main]
async fn main() {
    // Construct the qubit router
    let router = Router::new().handler(hello_world);

    // Save the type
    router.write_type_to_file("./bindings.ts");
    println!("Successfully write server type to `./bindings.ts`");

    // Create service and handle
    let (qubit_service, qubit_handle) = router.to_service(|_| ());

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
