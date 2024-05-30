# Hello, world!

The simplest Qubit setup possible.

In one terminal, start the server.

```sh
cargo run
```

In another terminal, install TypeScript dependencies, and run the client!

```sh
npm i
npm run start
```

## Note

The TypeScript client has some additional dependencies in order to get it up and running quickly,
namely `ws`. This is not required for clients that are running in the web browser due to
`WebSocket` existing, however in Node this is not the case.
