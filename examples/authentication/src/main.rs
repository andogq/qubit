use std::net::SocketAddr;

use axum::{
    Form,
    response::{IntoResponse, Response},
    routing::post,
};
use cookie::Cookie;
use hyper::{StatusCode, header::SET_COOKIE};
use qubit::{ErrorCode, Extensions, FromRequestExtensions, Router, RpcError, handler};
use serde::Deserialize;
use tokio::net::TcpListener;
use tower::ServiceBuilder;

const COOKIE_NAME: &str = "qubit-auth";

/// Don't do this
const USERNAME: &str = "user";
const PASSWORD: &str = "password";

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

/// Axum endpoint to handle form login
async fn login(Form(login_form): Form<LoginForm>) -> impl IntoResponse {
    if login_form.username == USERNAME && login_form.password == PASSWORD {
        Response::builder()
            .status(StatusCode::OK)
            .header(
                SET_COOKIE,
                Cookie::build((COOKIE_NAME, "abc-123"))
                    .path("/")
                    .same_site(cookie::SameSite::Lax)
                    .build()
                    .to_string(),
            )
            .body("login success".to_string())
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body("login fail".to_string())
            .unwrap()
    }
}

/// A simple context that optionally contains a cookie.
struct ReqCtx {
    auth_cookie: Option<Cookie<'static>>,
}

impl FromRequestExtensions<()> for ReqCtx {
    async fn from_request_extensions(
        _ctx: (),
        mut extensions: Extensions,
    ) -> Result<Self, RpcError> {
        Ok(Self {
            // Extract the auth cookie from the extensions
            auth_cookie: extensions.remove(),
        })
    }
}

/// Another context, used to represent an authenticated request. Will act as a middleware, as a
/// handler that relies on this context will only be run if it can be successfully generated.
struct AuthCtx {
    user: String,
}

impl FromRequestExtensions<()> for AuthCtx {
    async fn from_request_extensions(_ctx: (), extensions: Extensions) -> Result<Self, RpcError> {
        // Build up the `ReqCtx`
        let req_ctx = ReqCtx::from_request_extensions((), extensions).await?;

        // Enforce that the auth cookie is present
        let Some(cookie) = req_ctx.auth_cookie else {
            // Return an error to cancel the request if it's not
            return Err(RpcError {
                code: ErrorCode::ServerError(-32001),
                message: "Authentication required".to_string(),
                data: None,
            });
        };

        // Otherwise, progress using this new context.
        Ok(AuthCtx {
            user: cookie.value().to_string(),
        })
    }
}

/// Handler takes in [`ReqCtx`], so will run regardless of authentication status.
#[handler(query)]
async fn echo_cookie(ctx: ReqCtx) -> String {
    if let Some(cookie) = ctx.auth_cookie {
        format!("A cookie is set: {cookie}")
    } else {
        "No cookie is set".to_string()
    }
}

/// Handler takes in [`AuthCtx`], so will only run if the middleware can be properly constructed.
#[handler(query)]
async fn secret_endpoint(ctx: AuthCtx) -> String {
    format!("Welcome {}. The secret is: `super_secret`", ctx.user)
}

#[tokio::main]
async fn main() {
    // Create the qubit router
    let router = Router::<()>::new()
        .handler(echo_cookie)
        .handler(secret_endpoint);
    router.generate_type("./auth-demo/src/bindings.ts").unwrap();

    let (qubit_service, handle) = router.into_service(());

    let qubit_service = ServiceBuilder::new()
        .map_request(|mut req: hyper::Request<_>| {
            // Extract a certain cookie from the request
            let auth_cookie = req
                // Pull out the request headers
                .headers()
                // Select the cookie header
                .get(hyper::header::COOKIE)
                // Get the value of the header
                .and_then(|cookie| cookie.to_str().ok())
                .and_then(|cookie_header| {
                    // Parse the cookie header
                    Cookie::split_parse(cookie_header.to_string())
                        .filter_map(|cookie| cookie.ok())
                        // Attempt to find a specific cookie that matches the cookie we want
                        .find(|cookie| cookie.name() == COOKIE_NAME)
                });

            // If we find the auth cookie, save it to the request extension
            if let Some(auth_cookie) = auth_cookie {
                req.extensions_mut().insert(auth_cookie);
            }

            req
        })
        .service(qubit_service);

    // Once the handle is dropped the server will automatically shutdown, so leak it to keep it
    // running. Don't actually do this.
    Box::leak(Box::new(handle));

    // Create a simple axum router with the different implementations attached
    let axum_router = axum::Router::new()
        .route("/login", post(login))
        .nest_service("/rpc", qubit_service);

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
}
