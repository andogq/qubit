<div align="center">
  <img src="./logo.png" alt="" width="120" />
  <h1>Qubit: Seamless RPC For Rust & TypeScript</h1>

  <a href="https://crates.io/crates/qubit"><img src="https://img.shields.io/crates/v/qubit" alt="crates.io" /></a>
  <a href="https://docs.rs/qubit/latest/qubit"><img src="https://img.shields.io/docsrs/qubit" alt="docs.rs" /></a>
  <a href="https://www.npmjs.com/package/@qubit-rs/client"><img src="https://img.shields.io/npm/v/%40qubit-rs%2Fclient" alt="npm" /></a>
  <a href="https://github.com/andogq/qubit/actions/workflows/checks.yml"><img src="https://github.com/andogq/qubit/actions/workflows/checks.yml/badge.svg" alt="checks" /></a>
</div>

Tired of wrestling with RPC boilerplate? Qubit simplifies communication between your Rust services
and TypeScript clients, offering a type-safe and feature-rich development experience, so you can
focus on building amazing applications.

## Features:

- **Generated Type-Safe Clients**: Say goodbye to manual type definitions, Qubit automatically
  generates TypeScript clients based on your Rust API, ensuring a smooth development experience.

- **Subscriptions**: Build real-time, data-driven applications with subscriptions, allowing for
  your Rust server to push data directly to connected TypeScript clients.

- **Build Modular APIs**: Organise your API handlers into nested routers, ensuring simplicity and
  maintainability as your service grows.

- **Serde Compatibility**: Leverage Serde for seamless data serialisation and deserialisation
  between Rust and TypeScript.

- **Built on JSONRPC 2.0**: Need a non-TypeScript client? Use any JSONRPC client in any language
  over WebSockets or HTTP.

- **Proven Base**: Built on established libraries like
  [`ts-rs`](https://github.com/Aleph-Alpha/ts-rs) for type generation and
  [`jsonrpsee`](https://github.com/paritytech/jsonrpsee) as the JSONRPC implementation.

## Getting Started

1. Add the required dependencies

```toml
# Cargo.toml
[dependencies]
qubit = "0.6.1"

serde = { version = "1.0", features = ["derive"] } # Required for serialisable types
futures = "0.3.30" # Required for streaming functionality

tokio = { version = "1.38", features = ["full"] }
axum = "0.7"
hyper = { version = "1.0", features = ["server"] }
```

```bash
pnpm i @qubit-rs/client@latest
```

2. Setup a Qubit router, and save the generated types

```rs
#[handler(query)]
async fn hello_world() -> String {
    "Hello, world!".to_string()
}

let router = Router::new()
    .handler(hello_world);

router.write_bindings_to_dir("./bindings");
```

3. Attach the Qubit router to an Axum router, and start it

```rs
// Create a service and handle
let (qubit_service, qubit_handle) = router.to_service(());

// Nest into an Axum router
let axum_router = axum::Router::<()>::new()
    .nest_service("/rpc", qubit_service);

// Start a Hyper server
axum::serve(
    tokio::net::TcpListener::bind(&SocketAddr::from(([127, 0, 0, 1], 9944)))
        .await
        .unwrap(),
    axum_router,
)
.await
.unwrap();

qubit_handle.stop().unwrap();
```

4. Make requests from the TypeScript client

```ts
// Import transport from client, and generated server type
import { build_client, http } from "@qubit-rs/client";
import type { QubitServer } from "./bindings";

// Connect with the API
const api = build_client<QubitServer>(http("http://localhost:9944/rpc"));

// Call the handlers
const message = await api.hello_world.query();
console.log("received from server:", message);
```

## Examples

Checkout all the examples in the [`examples`](./examples) directory.

## Cargo Features

| Feature            | Description                                                                                                                               |
|--------------------|-------------------------------------------------------------------------------------------------------------------------------------------|
| `ts-format`        | Format generated TypeScript types. *Note:* This adds a lot of dependencies.                                                               |
| `ts-esm`           | Ensure `import` statements conform with the ES Modules spec by appending `.js` to paths. This likely isn't needed if a bundler is in use. |
| `ts-serde-json`    | Add TypeScript support for `serde_json`.                                                                                                  |
| `ts-chrono`        | Add TypeScript support for `chrono`.                                                                                                      |
| `ts-bigdecimal`    | Add TypeScript support for `bigdecimal`.                                                                                                  |
| `ts-url`           | Add TypeScript support for `url`.                                                                                                         |
| `ts-uuid`          | Add TypeScript support for `uuid`.                                                                                                        |
| `ts-bson-uuid`     | Add TypeScript support for `bson-uuid`.                                                                                                   |
| `ts-bytes`         | Add TypeScript support for `bytes`.                                                                                                       |
| `ts-indexmap`      | Add TypeScript support for `indexmap`.                                                                                                    |
| `ts-ordered-float` | Add TypeScript support for `ordered-float`.                                                                                               |
| `ts-heapless`      | Add TypeScript support for `heapless`.                                                                                                    |
| `ts-semver`        | Add TypeScript support for `semver`.                                                                                                      |
| `ts-smol-str`      | Add TypeScript support for `smol_str`.                                                                                                    |
| `ts-tokio`         | Add TypeScript support for `tokio`.                                                                                                       |

## FAQs

### Qubit?

The term "Qubit" refers to the fundamental unit of quantum information. Just as a qubit can exist
in a superposition of states, Qubit bridges the gap between Rust and TypeScript, empowering
developers to create truly exceptional applications.

## Prior Art

- [`rspc`](https://github.com/oscartbeaumont/rspc): Similar concept, however uses a bespoke
solution for generating TypeScript types from Rust structs, which isn't completely compatible with
all of Serde's features for serialising and deserialising structs.

- [`trpc`](https://github.com/trpc/trpc): Needs no introduction, however it being restricted to
TypeScript backends makes it relatively useless for Rust developers.
