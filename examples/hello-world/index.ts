// Import transport from client, and generated server type
import { http, build_client } from "@qubit-rs/client";
import type { QubitServer } from "./bindings";

async function main() {
  // Connect with the API
  const api = build_client<QubitServer>(http("http://localhost:9944/rpc"));

  // Call the handlers
  const message = await api.hello_world.query();
  console.log("recieved from server:", message);
}

main();
