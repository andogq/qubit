# rstrpc - Rust/TypeScript RPC

Generate type-safe TypeScript clients for your Rust APIs, with Serde compatibility, subscriptions,
and more!

- [`npm`](https://www.npmjs.com/package/@rstrpc/client)
- [`crates.io`](https://crates.io/crates/rstrpc)

## Features:

- Context based middleware

- Subscriptions

- Nested routers

- Serde compatibility for serialising and deserialising parameters and return values

- Standardised JSONRPC 2.0 implementation, including `ws` and `http` transport layers

- Built using [`ts-rs`](https://github.com/Aleph-Alpha/ts-rs) and
[`jsonrpsee`](https://github.com/paritytech/jsonrpsee)

## Example

Check out the `example` directory for a full example.

### Rust Server

```rs
#[derive(Clone, Default)]
pub struct Ctx {
    count: Arc<AtomicUsize>,
}

// Handlers are defined as functions, where the function name will be the name of the handler
#[handler]
async fn hello_world(_ctx: Ctx) -> String {
    "Hello, world!".to_string()
}

// Handlers have access to the app state
#[handler]
async fn count(ctx: Ctx) -> usize {
    ctx.count.fetch_add(1, Ordering::Relaxed)
}

// Handlers can accept parameters
#[handler]
async fn count_by(ctx: Ctx, amount: usize) -> usize {
    ctx.count.fetch_add(amount, Ordering::Relaxed)
}

// Handlers can return a stream, in order to act as a subscription
#[handler(subscription)]
async fn countdown(ctx: AppCtx, min: usize, max: usize) -> impl Stream<Item = usize> {
    stream::iter(min..=max).then(|n| async move {
        n
    })
}

#[tokio::main]
async fn main() {
    // Build the app, attaching handlers as required
    let app = Router::new()
        .handler(hello_world)
        .handler(count);

    // Create a stop channel so that the server can be programatically terminated
    let (stop_handle, server_handle) = stop_channel();

    // Global app context
    let ctx = Ctx::default();

    // Create or nest app into an existing axum server
    let router = axum::Router::<()>::new()
        .route("/", get(|| async { "another endpoint!" }))
        .nest_service("/rpc", app.to_service(move |_| {
            // For each request that comes in, clone the context so that it can be shared around
            ctx.clone()
        }, stop_handle));

    // Start the axum rounter as normal
    hyper::Server::bind(&SocketAddr::from([127, 0, 0, 1], 9944))
        .serve(router.into_make_service())
        .await
        .unwrap();

    // Upon termination of the hyper server, properly shutdown the RPC server
    server_handle.stop().unwrap();
}
```

### TypeScript

```ts
import { ws } from "@rstrpc/client";

// This type is automatically generated based on the Rust API
import type { Server } from "./bindings.ts";

// Start a new client, passing the type as a generic parameter
const client = ws<Server>("ws://localhost:9944/rpc");

// Handlers can be accessed from the client just by calling the method!
const message = await client.hello_world();
console.log(message); // "Hello, world!"

for (let i = 0; i < 5; i++) {
    const count = await client.count();
    console.log(`The count is: ${count}`);
}

// Parameters are typed, and are passed as if it were a regular function
await client.count_by(10);

// Subscriptions are just like regular handlers, except they also accept life-cycle handlers for
// data, errors, and subcription end
await client.countdown(1, 4).subscribe({
	on_data: (data) => {
		console.log(`Countdown: ${data}`);
	},
	on_end: () => {
		console.log("Countdown done!");
	}
});
```

## Acknowledgements

- [`rspc`](https://github.com/oscartbeaumont/rspc): Similar concept, however uses a bespoke
solution for generating TypeScript types from Rust structs, which isn't completely compatible with
all of Serde's features for serialising and deserialising structs.

- [`trpc`](https://github.com/trpc/trpc): Needs no introduction, however it being restricted to
TypeScript backends makes it relatively useless for Rust developers.
