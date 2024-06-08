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

  // Do some maths
  for (let i = 0; i < 5; i++) {
    await api.increment.mutate();
  }
  console.log("The value is", await api.get.query());

  for (let i = 0; i < 3; i++) {
    await api.decrement.mutate();
  }
  console.log("The value is", await api.get.query());

  await api.add.mutate(10);
  console.log("The value is", await api.get.query());

  console.log("=== Beginning Countdown ===");
  await new Promise<void>((resolve) => {
    api.countdown.subscribe({
      on_data: (n) => {
        console.log(`${n}...`);
      },
      on_end: () => {
        resolve();
      },
    });
  });
  console.log("=== Lift Off! ===");
}

main();
