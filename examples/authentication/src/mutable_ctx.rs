use std::sync::Arc;

use qubit::{handler, ErrorCode, FromContext, Router, RpcError};
use tokio::sync::RwLock;

/// Don't do this
const USERNAME: &str = "user";
const PASSWORD: &str = "password";

#[derive(Clone, Default)]
struct Ctx {
    authed: Arc<RwLock<bool>>,
}

#[derive(Clone)]
struct AuthCtx;

impl FromContext<Ctx> for AuthCtx {
    /// Implementation to generate the [`AuthCtx`] from the [`ReqCtx`]. Is falliable, so requests
    /// can be blocked at this point.
    async fn from_app_ctx(ctx: Ctx) -> Result<Self, qubit::RpcError> {
        // Enforce that the auth cookie is present
        if !*ctx.authed.read().await {
            // Return an error to cancel the request if it's not
            return Err(RpcError {
                code: ErrorCode::ServerError(-32001),
                message: "Authentication required".to_string(),
                data: None,
            });
        };

        // Otherwise, progress using this new context.
        Ok(AuthCtx)
    }
}

#[handler(mutation)]
async fn login(ctx: Ctx, username: String, password: String) -> bool {
    if username == USERNAME && password == PASSWORD {
        // Update the context to indicate that the user is logged in
        *ctx.authed.write().await = true;
        return true;
    }

    false
}

#[handler(query)]
async fn secret_endpoint() -> String {
    "Secret message!".to_string()
}

pub fn init() -> axum::Router<()> {
    let router = Router::new().handler(login).handler(secret_endpoint);
    router.write_bindings_to_dir("./auth-demo/src/bindings-mutable-ctx");

    let (qubit_service, handle) =
        router.to_service(move |_| async { Ctx::default() }, |_| async {});

    // Once the handle is dropped the server will automatically shutdown, so leak it to keep it
    // running. Don't actually do this.
    Box::leak(Box::new(handle));

    axum::Router::new().nest_service("/rpc", qubit_service)
}
