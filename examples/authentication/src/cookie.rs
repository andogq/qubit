use axum::{
    response::{IntoResponse, Response},
    routing::post,
    Form,
};
use cookie::Cookie;
use hyper::{
    header::{COOKIE, SET_COOKIE},
    StatusCode,
};
use qubit::{handler, ErrorCode, FromContext, Router, RpcError};
use serde::Deserialize;

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

/// Context which will be generated for every request.
#[derive(Clone)]
struct ReqCtx {
    /// Authentication cookie extracted from the headers, if present.
    auth_cookie: Option<String>,
}

/// Another context, used to represent an authenticated request. Will act as a middleware, as a
/// handler that relies on this context will only be run if it can be successfully generated.
#[derive(Clone)]
struct AuthCtx {
    user: String,
}

impl FromContext<ReqCtx> for AuthCtx {
    /// Implementation to generate the [`AuthCtx`] from the [`ReqCtx`]. Is falliable, so requests
    /// can be blocked at this point.
    async fn from_app_ctx(ctx: ReqCtx) -> Result<Self, qubit::RpcError> {
        // Enforce that the auth cookie is present
        let Some(cookie) = ctx.auth_cookie else {
            // Return an error to cancel the request if it's not
            return Err(RpcError {
                code: ErrorCode::ServerError(-32001),
                message: "Authentication required".to_string(),
                data: None,
            });
        };

        // Otherwise, progress using this new context.
        Ok(AuthCtx { user: cookie })
    }
}

/// Handler takes in [`ReqCtx`], so will run regardless of authentication status.
#[handler]
async fn echo_cookie(ctx: ReqCtx) -> String {
    if let Some(cookie) = ctx.auth_cookie {
        format!("A cookie is set: {cookie}")
    } else {
        "No cookie is set".to_string()
    }
}

/// Handler takes in [`AuthCtx`], so will only run if the middleware can be properly constructed.
#[handler]
async fn secret_endpoint(ctx: AuthCtx) -> String {
    format!("Welcome {}. The secret is: `super_secret`", ctx.user)
}

pub fn init() -> axum::Router<()> {
    let router = Router::new().handler(echo_cookie).handler(secret_endpoint);
    router.write_bindings_to_dir("./auth-demo/src/bindings-cookie-auth");

    let (qubit_service, handle) = router.to_service(
        move |req| {
            // Extract cookie from request
            let auth_cookie = req
                .headers()
                .get_all(COOKIE)
                .into_iter()
                .flat_map(|cookie| Cookie::split_parse(cookie.to_str().unwrap()))
                .flatten()
                .find(|cookie| cookie.name() == COOKIE_NAME)
                .map(|cookie| cookie.value().to_string());

            async {
                // Attach it into the request context
                ReqCtx { auth_cookie }
            }
        },
        |_| async {},
    );

    // Once the handle is dropped the server will automatically shutdown, so leak it to keep it
    // running. Don't actually do this.
    Box::leak(Box::new(handle));

    axum::Router::new()
        .route("/login", post(login))
        .nest_service("/rpc", qubit_service)
}
