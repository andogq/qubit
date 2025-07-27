use std::net::SocketAddr;

use qubit::{Router, handler};
use tokio::net::TcpListener;

#[handler(query)]
async fn hello_world() -> String {
    "Hello, world!".to_string()
}

#[tokio::main]
async fn main() {
    // Construct the qubit router
    let router = Router::new().handler(hello_world);

    // Save the type
    router.generate_type("./bindings.ts").unwrap();
    println!("Successfully write server type to `./bindings`");

    // Create service and handle
    let (qubit_service, qubit_handle) = router.into_service(());

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
