use std::net::SocketAddr;

mod cookie;
mod mutable_ctx;

#[tokio::main]
async fn main() {
    // Create a simple axum router with the different implementations attached
    let axum_router = axum::Router::<()>::new()
        .nest("/cookie", cookie::init())
        .nest("/mutable-ctx", mutable_ctx::init());

    // Start a Hyper server
    println!("Listening at 127.0.0.1:9944");
    hyper::Server::bind(&SocketAddr::from(([127, 0, 0, 1], 9944)))
        .serve(axum_router.into_make_service())
        .await
        .unwrap();
}
