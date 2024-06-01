// Import transport from client, and generated server type
import { ws } from "@qubit-rs/client";
import type { QubitServer } from "./bindings";

// Polyfill only required for running in NodeJS
import { WebSocket } from "ws";

async function main() {
  // Connect with the API
  const api = ws<QubitServer>(
    "ws://localhost:9944/rpc",
    // @ts-ignore mis-matching WebSocket definitions
    { WebSocket },
  );

  // Call the handlers
  const message = await api.hello_world();
  console.log("recieved from server:", message);
}

main();
