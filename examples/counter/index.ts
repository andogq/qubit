// Import transport from client, and generated server type
import { ws } from "@qubit-rs/client";
import type { QubitServer } from "./bindings.ts";

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
    await api.increment();
  }
  console.log("The value is", await api.get());

  for (let i = 0; i < 3; i++) {
    await api.decrement();
  }
  console.log("The value is", await api.get());

  await api.add(10);
  console.log("The value is", await api.get());

  console.log("=== Beginning Countdown ===");
  await new Promise<void>((resolve) => {
    api.countdown().subscribe({
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
